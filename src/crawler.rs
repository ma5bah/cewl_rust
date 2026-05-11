//! Core crawl loop — mirrors Ruby MySpider / MySpiderInstance behaviour.

use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashSet;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

use crate::capture;
use crate::cli::CewlArgs;
use crate::fetcher::browser_fetcher::{BrowserCfg, BrowserFetcher, RenderWait};
use crate::fetcher::static_fetcher::{StaticConfig, StaticFetcher};
use crate::fetcher::FetchOutcome;
use crate::html;
use crate::metadata;
use crate::tree::Tree;
use crate::words;

/// Normalise raw headers arg ("Name: value") into pairs.
fn parse_header_arg(s: &str) -> Option<(String, String)> {
    let mut parts = s.splitn(2, ':');
    let k = parts.next()?.trim().to_string();
    let v = parts.next()?.trim().to_string();
    if k.is_empty() {
        None
    } else {
        Some((k, v))
    }
}

/// Build a proxy URL from host/port/user/pass components.
fn build_proxy_url(host: &str, port: u16, user: Option<&str>, pass: Option<&str>) -> String {
    match (user, pass) {
        (Some(u), Some(p)) => format!("http://{u}:{p}@{host}:{port}"),
        _ => format!("http://{host}:{port}"),
    }
}

/// URL admission filter (mirrors `add_url_check` in Ruby).
struct UrlFilter {
    base_url: Url,
    offsite: bool,
    exclude: HashSet<String>,
    allowed: Option<Regex>,
}

impl UrlFilter {
    fn new(base_url: &str, args: &CewlArgs) -> Self {
        let mut exclude = HashSet::new();
        if let Some(ref p) = args.exclude {
            if let Ok(content) = std::fs::read_to_string(p) {
                for line in content.lines() {
                    let t = line.trim();
                    if !t.is_empty() {
                        exclude.insert(t.to_string());
                    }
                }
            }
        }
        let allowed = args.allowed.as_deref().and_then(|p| Regex::new(p).ok());

        Self {
            base_url: Url::parse(base_url).expect("bad seed url"),
            offsite: args.offsite,
            exclude,
            allowed,
        }
    }

    /// True if this URL should be fetched.
    fn admit(&self, url_str: &str) -> bool {
        // Drop anchors and graphics
        let lower = url_str.to_ascii_lowercase();
        if lower.starts_with('#')
            || lower.ends_with(".zip")
            || lower.ends_with(".gz")
            || lower.ends_with(".bz2")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
        {
            return false;
        }
        let Ok(parsed) = Url::parse(url_str) else {
            return false;
        };
        if parsed.scheme() == "mailto" {
            return false;
        }
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return false;
        }
        if !self.offsite
            && (parsed.host_str() != self.base_url.host_str()
                || parsed.port_or_known_default() != self.base_url.port_or_known_default()
                || parsed.scheme() != self.base_url.scheme())
        {
            return false;
        }
        let req_uri = parsed.path().to_string();
        if self.exclude.contains(&req_uri) {
            return false;
        }
        if let Some(ref re) = self.allowed {
            if !re.is_match(parsed.path()) {
                return false;
            }
        }
        true
    }

    /// True if the URL is a mailto: link.
    fn is_mailto(url_str: &str) -> bool {
        url_str.to_ascii_lowercase().starts_with("mailto:")
    }

    /// True if extension is a binary/document type (skip HTML pipeline).
    fn is_binary_ext(url_str: &str) -> bool {
        let lower = url_str.to_ascii_lowercase();
        let path = lower.split('?').next().unwrap_or(&lower);
        let ext = path.rsplit('.').next().unwrap_or("");
        matches!(
            ext,
            "doc"
                | "dot"
                | "docx"
                | "docm"
                | "dotx"
                | "dotm"
                | "ppt"
                | "pot"
                | "pptx"
                | "pptm"
                | "potx"
                | "potm"
                | "pps"
                | "ppam"
                | "xls"
                | "xlt"
                | "xlsx"
                | "xlsm"
                | "xltx"
                | "xltm"
                | "xlsb"
                | "xlam"
                | "pdf"
                | "zip"
                | "gz"
                | "bz2"
        )
    }
}

pub struct CrawlResult {
    pub words: IndexMap<String, u64>,
    pub groups: IndexMap<String, u64>,
    pub emails: Vec<String>,
    pub usernames: Vec<String>,
}

/// Email regex (line-by-line, same as Ruby).
static EMAIL_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)\b[-A-Z0-9._%+]+@(?:[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?\.)+[A-Z]{2,63}\b")
        .unwrap()
});

fn extract_emails_from_text(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in text.lines() {
        let mut rest = line.to_string();
        while let Some(m) = EMAIL_RE.find(&rest) {
            found.push(m.as_str().to_string());
            let end = m.end();
            rest = rest[end..].to_string();
        }
    }
    found
}

pub async fn run(args: &CewlArgs) -> anyhow::Result<CrawlResult> {
    let seed = normalise_url(&args.url);
    let filter = UrlFilter::new(&seed, args);

    // Build static fetcher (always available as fallback)
    let headers: Vec<(String, String)> = args
        .header
        .iter()
        .filter_map(|h| parse_header_arg(h))
        .collect();

    let proxy_url = args.proxy_host.as_deref().map(|h| {
        build_proxy_url(
            h,
            args.proxy_port_or_default(),
            args.proxy_username.as_deref(),
            args.proxy_password.as_deref(),
        )
    });

    let auth_type = args.auth_type.as_deref().unwrap_or("").to_ascii_lowercase();

    let static_cfg = StaticConfig {
        user_agent: args.ua.clone(),
        headers: headers.clone(),
        proxy_url: proxy_url.clone(),
        insecure: args.insecure,
        auth_basic: if auth_type == "basic" {
            Some((
                args.auth_user.clone().unwrap_or_default(),
                args.auth_pass.clone().unwrap_or_default(),
            ))
        } else {
            None
        },
        auth_digest: if auth_type == "digest" {
            Some((
                args.auth_user.clone().unwrap_or_default(),
                args.auth_pass.clone().unwrap_or_default(),
            ))
        } else {
            None
        },
        timeout: std::time::Duration::from_secs(60),
    };
    let static_fetcher = Arc::new(StaticFetcher::new(static_cfg)?);

    // Build browser fetcher (if render mode on)
    let browser_fetcher: Option<Arc<BrowserFetcher>> = if args.render {
        let bcfg = BrowserCfg {
            browser_path: args.browser_path.clone(),
            user_data_dir: args.browser_user_data_dir.clone(),
            profile_directory: args.browser_profile_dir.clone(),
            headed: args.browser_headed(),
            no_sandbox: args.no_sandbox,
            user_agent: args.ua.clone(),
            proxy: proxy_url,
            insecure: args.insecure,
            render_wait: RenderWait::parse(&args.render_wait),
            render_timeout: std::time::Duration::from_secs(args.render_timeout),
            human_in_loop: args.human_in_loop,
            human_timeout: std::time::Duration::from_secs(args.human_timeout),
            concurrency: args.concurrency,
            extra_headers: headers,
        };
        match BrowserFetcher::launch(bcfg).await {
            Ok(bf) => {
                if args.verbose {
                    eprintln!("[cewl] Browser launched");
                }
                Some(Arc::new(bf))
            }
            Err(e) => {
                eprintln!(
                    "[cewl] Warning: browser launch failed ({e}), falling back to static HTTP"
                );
                None
            }
        }
    } else {
        None
    };

    // Shared state
    let mut tree = Tree::new(None, String::new(), 0, args.debug);
    tree.max_depth = args.depth;
    tree.push(None, seed.clone());

    let mut seen: HashSet<String> = HashSet::new();
    let mut word_hash: IndexMap<String, u64> = IndexMap::new();
    let mut group_hash: IndexMap<String, u64> = IndexMap::new();
    let mut emails: Vec<String> = Vec::new();
    let mut usernames: Vec<String> = Vec::new();
    let meta_dir = args.meta_temp_dir_str();
    let mut pages_done: u64 = 0;
    let max_pages = args.max_pages.unwrap_or(0);

    let interrupted = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let flag = interrupted.clone();
        let _ = ctrlc::set_handler(move || {
            eprintln!("\nHold on, about to stop...");
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });
    }

    while !tree.empty() {
        if interrupted.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        if max_pages > 0 && pages_done >= max_pages {
            break;
        }

        let Some(pair) = tree.pop() else { break };
        // pair is {prior_url => next_url}
        let Some((prior, page_url)) = pair.into_iter().next() else {
            break;
        };

        // Skip already-seen
        if seen.contains(&page_url) {
            continue;
        }
        // Capture mailto emails
        if UrlFilter::is_mailto(&page_url) {
            if args.email {
                let addr = page_url.trim_start_matches("mailto:").to_string();
                if !addr.is_empty() {
                    emails.push(addr);
                }
            }
            continue;
        }
        // Admission check
        if !filter.admit(&page_url) {
            if args.debug {
                eprintln!("[filter] skip {page_url}");
            }
            continue;
        }
        seen.insert(page_url.clone());

        if args.verbose {
            match &prior {
                None => eprintln!("Visiting: {page_url}"),
                Some(p) => eprintln!("Visiting: {page_url} referred from {p}"),
            }
        }

        // Decide fetcher: binary ext or digest auth → static, else browser
        let use_browser = browser_fetcher.is_some()
            && !UrlFilter::is_binary_ext(&page_url)
            && auth_type != "digest";

        let outcome: FetchOutcome = if use_browser {
            let bf = browser_fetcher.as_ref().unwrap().clone();
            let u = page_url.clone();
            let v = args.verbose;
            let result = bf.fetch(&u, v).await;
            // Fallback on error unless --no-fallback
            match result {
                FetchOutcome::Error(ref e) if !args.no_fallback => {
                    if args.verbose {
                        eprintln!("[browser] error ({e}), falling back to static");
                    }
                    static_fetcher.fetch(&page_url).await
                }
                other => other,
            }
        } else {
            static_fetcher.fetch(&page_url).await
        };

        // Handle redirect: enqueue destination
        if let FetchOutcome::Redirect { ref from, ref to } = outcome {
            if args.debug {
                eprintln!("[redirect] {from} -> {to}");
            }
            tree.push(Some(from.clone()), to.clone());
            continue;
        }

        // Enqueue extra browser-sniffed URLs
        for sniffed_url in outcome.discovered_urls() {
            if filter.admit(sniffed_url) && !seen.contains(sniffed_url) {
                tree.push(Some(page_url.clone()), sniffed_url.clone());
            }
        }

        // Process body
        let Some((final_url, body, ct)) = outcome.body_and_url() else {
            continue;
        };

        pages_done += 1;

        // Capture URL-structure into wordlist
        if args.capture_paths || args.capture_subdomains || args.capture_domain {
            for component in capture::extract_path_components(&final_url) {
                if component.len() >= args.min_word_length {
                    *word_hash.entry(component).or_insert(0) += 1;
                }
            }
            if args.capture_subdomains {
                for sub in capture::extract_subdomain_components(&final_url) {
                    if sub.len() >= args.min_word_length {
                        *word_hash.entry(sub).or_insert(0) += 1;
                    }
                }
            }
            if args.capture_domain {
                if let Some(dom) = capture::extract_registrable_domain(&final_url) {
                    if dom.len() >= args.min_word_length {
                        *word_hash.entry(dom).or_insert(0) += 1;
                    }
                }
            }
        }

        // Binary/document branch
        let is_bin_ext = UrlFilter::is_binary_ext(&page_url);
        let is_bin_ct = {
            let lower = ct.as_deref().unwrap_or("").to_ascii_lowercase();
            lower.contains("pdf")
                || lower.contains("msword")
                || lower.contains("officedocument")
                || lower.contains("octet-stream")
                || lower.contains("zip")
        };

        if (is_bin_ext || is_bin_ct) && args.meta {
            let fname = if args.keep {
                page_url
                    .rsplit('/')
                    .next()
                    .unwrap_or("cewl_tmp")
                    .to_string()
            } else {
                let ext = page_url.rsplit('.').next().unwrap_or("");
                if ext.is_empty() {
                    "cewl_tmp".to_string()
                } else {
                    format!("cewl_tmp.{ext}")
                }
            };
            let fpath = PathBuf::from(format!("{meta_dir}{fname}"));
            if let Ok(mut f) = std::fs::File::create(&fpath) {
                let _ = f.write_all(&body);
                if let Some(meta) = metadata::process_file(&fpath, args.verbose) {
                    usernames.extend(meta);
                }
            }
            continue;
        }

        // HTML pipeline
        let html_str = html::decode_body_bytes(&body, ct.as_deref());

        // Extract hrefs for next URLs
        let hrefs = html::extract_hrefs(&html_str, &final_url);
        for href in hrefs {
            if UrlFilter::is_mailto(&href) && args.email {
                let addr = href.trim_start_matches("mailto:").to_string();
                if !addr.is_empty() {
                    emails.push(addr);
                }
            } else if filter.admit(&href) && !seen.contains(&href) {
                tree.push(Some(final_url.clone()), href);
            }
        }

        // JS redirect extract (static path only)
        if browser_fetcher.is_none() || !use_browser {
            for jr in html::extract_js_redirects(&html_str) {
                if let Some(resolved) = html::resolve_href(&final_url, &jr) {
                    if filter.admit(&resolved) && !seen.contains(&resolved) {
                        tree.push(Some(final_url.clone()), resolved);
                    }
                }
            }
        }

        if !args.wordlist_enabled() && !args.email {
            continue;
        }

        // Strip scripts/styles, harvest attributes, strip tags
        let mut body_text =
            html::strip_scripts_styles_simple(&html_str, args.keep_js, args.keep_css);
        body_text.push_str(&html::extract_meta_append(&html_str));
        body_text.push_str(&html::harvest_alt_title(&html_str));
        html::strip_comments(&mut body_text);
        let stripped = html::strip_html_tags(&body_text);
        let words_raw = html::decode_html_entities(&stripped);

        // Email extraction
        if args.email {
            for e in extract_emails_from_text(&words_raw) {
                emails.push(e);
            }
        }

        // Word extraction
        if args.wordlist_enabled() {
            let normalised = words::normalize_words(
                &words_raw,
                args.lowercase,
                args.with_numbers,
                args.convert_umlauts,
            );
            words::count_words(
                &normalised,
                args.min_word_length,
                args.max_word_length,
                &mut word_hash,
                args.groups,
                &mut group_hash,
            );
        }
    }

    // Deduplicate emails
    emails.sort();
    emails.dedup();
    emails.retain(|e| !e.trim().is_empty());

    // Deduplicate usernames
    usernames.sort();
    usernames.dedup();
    usernames.retain(|u| !u.trim().is_empty());

    Ok(CrawlResult {
        words: word_hash,
        groups: group_hash,
        emails,
        usernames,
    })
}

/// Ensure URL has a scheme.
fn normalise_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{url}")
    }
}
