//! HTML parsing: links, meta, stripping, entities (Ruby `cewl.rb` pipeline).

use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;

static HREF: LazyLock<Selector> = LazyLock::new(|| Selector::parse("a[href]").unwrap());
static META_DESC: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"meta[name="description" i]"#).unwrap());
static META_KW: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"meta[name="keywords" i]"#).unwrap());

/// Ruby: if path empty base = full a_url else chop at last `/`.
pub fn base_url_for_page(a_url: &str) -> String {
    if let Ok(u) = url::Url::parse(a_url) {
        if u.path().is_empty() || u.path() == "/" {
            return a_url.to_string();
        }
    }
    a_url
        .rfind('/')
        .map(|i| a_url[..i].to_string())
        .unwrap_or_else(|| a_url.to_string())
}

pub fn resolve_href(base: &str, href: &str) -> Option<String> {
    let base_u = url::Url::parse(base).ok()?;
    let joined = base_u.join(href).ok()?;
    if joined.fragment().is_some_and(|f| f.is_empty()) {
        return None;
    }
    Some(joined.as_str().to_string())
}

pub fn extract_hrefs(html: &str, page_url: &str) -> Vec<String> {
    let base = base_url_for_page(page_url);
    let document = Html::parse_document(html);
    let mut out = Vec::new();
    for el in document.select(&HREF) {
        let Some(href) = el.value().attr("href") else {
            continue;
        };
        if let Some(u) = resolve_href(&base, href) {
            out.push(u);
        }
    }
    out
}

pub fn extract_meta_append(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut extra = String::new();
    for sel in [&*META_DESC, &*META_KW] {
        for el in document.select(sel) {
            if let Some(c) = el.value().attr("content") {
                if !c.is_empty() {
                    extra.push(' ');
                    extra.push_str(c);
                }
            }
        }
    }
    extra
}

/// Remove script/style by string replacement (approximate Ruby `dom.css(...).remove`).
pub fn strip_scripts_styles_simple(html: &str, keep_js: bool, keep_css: bool) -> String {
    let mut s = html.to_string();
    if !keep_js {
        let re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
        s = re.replace_all(&s, "").to_string();
    }
    if !keep_css {
        let re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
        s = re.replace_all(&s, "").to_string();
    }
    s
}

static JS_REDIRECT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(location\.href\s*=\s*['"])([^'"]*)(['"]\s*;)"#).unwrap());

pub fn extract_js_redirects(body: &str) -> Vec<String> {
    let mut v = Vec::new();
    for cap in JS_REDIRECT.captures_iter(body) {
        if let Some(u) = cap.get(2) {
            v.push(u.as_str().to_string());
        }
    }
    v
}

static ATTR_ALT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)alt="([^"]*)""#).unwrap());
static ATTR_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)title="([^"]*)""#).unwrap());

pub fn harvest_alt_title(body: &str) -> String {
    let mut t = String::new();
    for re in [&*ATTR_ALT, &*ATTR_TITLE] {
        for cap in re.captures_iter(body) {
            if let Some(m) = cap.get(1) {
                t.push(' ');
                t.push_str(m.as_str());
            }
        }
    }
    t
}

static COMMENT_BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());

/// Strip HTML comments (replace with a space so words on either side don't fuse).
pub fn strip_comments(body: &mut String) {
    *body = COMMENT_BLOCK.replace_all(body, " ").to_string();
}

/// Strip HTML tags, replacing each tag with a single space so adjacent text
/// nodes remain word-separated (Ruby's Nokogiri serialiser preserves whitespace
/// at element boundaries; the naive Rust regex strip needs to mimic that).
pub fn strip_html_tags(body: &str) -> String {
    TAGS.replace_all(body, " ").to_string()
}

pub fn decode_html_entities(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

/// Ruby UTF-8 round-trip invalid replace: approximate with lossy UTF-8.
pub fn decode_body_bytes(bytes: &[u8], content_type: Option<&str>) -> String {
    let mut label = encoding_rs::UTF_8;
    if let Some(ct) = content_type {
        if let Some(pos) = ct.to_ascii_lowercase().find("charset=") {
            let ch = ct[pos + "charset=".len()..].trim();
            let ch = ch.trim_matches(|c| c == '"' || c == '\'');
            if let Some(enc) = encoding_rs::Encoding::for_label(ch.as_bytes()) {
                label = enc;
            }
        }
    }
    let (cow, _, _) = label.decode(bytes);
    let mut s = cow.into_owned();
    if s.chars().any(|c| c == '\u{FFFD}') {
        let (cow2, _, _) = encoding_rs::UTF_8.decode(bytes);
        s = cow2.into_owned();
    }
    s
}
