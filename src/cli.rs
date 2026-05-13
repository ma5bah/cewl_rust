//! Command-line arguments for `cewl` and `fab`.

use clap::Parser;
use std::path::PathBuf;

pub const VERSION: &str = "6.3.1-rust";

#[derive(Clone, Debug, Parser)]
#[command(
    name = "cewl",
    version = VERSION,
    about = "Custom Word List generator (Rust port of CeWL)",
    after_help = "Files Already Bagged (local metadata):  cewl fab [OPTIONS] <FILES>...\nStandalone fab binary remains available for scripting."
)]
pub struct CewlArgs {
    #[arg(long, short = 'k', help = "Keep the downloaded file (metadata temp)")]
    pub keep: bool,

    #[arg(long, short = 'd', default_value_t = 2, help = "Depth to spider to")]
    pub depth: i32,

    #[arg(
        long = "min-word-length",
        alias = "min_word_length",
        short = 'm',
        default_value_t = 3,
        help = "Minimum word length"
    )]
    pub min_word_length: usize,

    #[arg(
        long = "max-word-length",
        alias = "max_word_length",
        short = 'x',
        help = "Maximum word length (unset = no max)"
    )]
    pub max_word_length: Option<usize>,

    #[arg(long, short = 'n', help = "Don't output the wordlist")]
    pub no_words: bool,

    #[arg(long, short = 'g', default_value_t = -1, help = "Return groups of N words (-1 = off)")]
    pub groups: i32,

    #[arg(long, short = 'o', help = "Let the spider visit other sites")]
    pub offsite: bool,

    #[arg(long, help = "File containing paths to exclude")]
    pub exclude: Option<PathBuf>,

    #[arg(long, help = "Regex that path must match to be followed")]
    pub allowed: Option<String>,

    #[arg(long, short = 'w', help = "Write word output to file")]
    pub write: Option<PathBuf>,

    #[arg(long, short = 'u', help = "User-Agent")]
    pub ua: Option<String>,

    #[arg(
        long,
        default_value = "/tmp/",
        help = "Temp dir for metadata downloads"
    )]
    pub meta_temp_dir: PathBuf,

    #[arg(
        long = "meta-file",
        alias = "meta_file",
        help = "Output file for meta data"
    )]
    pub meta_file: Option<PathBuf>,

    #[arg(
        long = "email-file",
        alias = "email_file",
        help = "Output file for email addresses"
    )]
    pub email_file: Option<PathBuf>,

    #[arg(long, help = "Lowercase all parsed words")]
    pub lowercase: bool,

    #[arg(long, help = "Accept words with numbers")]
    pub with_numbers: bool,

    #[arg(long, help = "Convert Latin-1 umlauts")]
    pub convert_umlauts: bool,

    #[arg(long, short = 'a', help = "Include metadata from documents")]
    pub meta: bool,

    #[arg(long, short = 'e', help = "Include email addresses")]
    pub email: bool,

    #[arg(long, help = "Add URL path components to wordlist")]
    pub capture_paths: bool,

    #[arg(long, help = "Add subdomain components to wordlist")]
    pub capture_subdomains: bool,

    #[arg(long, help = "Add registrable domain to wordlist")]
    pub capture_domain: bool,

    #[arg(long, help = "Capture paths, subdomains, and domain")]
    pub capture_url_structure: bool,

    #[arg(long, short = 'c', help = "Show count per word")]
    pub count: bool,

    #[arg(long = "auth-user", alias = "auth_user", help = "HTTP auth username")]
    pub auth_user: Option<String>,

    #[arg(long = "auth-pass", alias = "auth_pass", help = "HTTP auth password")]
    pub auth_pass: Option<String>,

    #[arg(
        long = "auth-type",
        alias = "auth_type",
        help = "HTTP auth type: basic or digest"
    )]
    pub auth_type: Option<String>,

    #[arg(long, short = 'H', help = "Extra header name:value (repeatable)")]
    pub header: Vec<String>,

    #[arg(long = "proxy-host", alias = "proxy_host", help = "Proxy host")]
    pub proxy_host: Option<String>,

    #[arg(
        long = "proxy-port",
        alias = "proxy_port",
        help = "Proxy port (default 8080 if host set)"
    )]
    pub proxy_port: Option<u16>,

    #[arg(
        long = "proxy-username",
        alias = "proxy_username",
        help = "Proxy username"
    )]
    pub proxy_username: Option<String>,

    #[arg(
        long = "proxy-password",
        alias = "proxy_password",
        help = "Proxy password"
    )]
    pub proxy_password: Option<String>,

    #[arg(long, short = 'v', help = "Verbose")]
    pub verbose: bool,

    #[arg(long, help = "Keep JavaScript in HTML")]
    pub keep_js: bool,

    #[arg(long, help = "Keep CSS in HTML")]
    pub keep_css: bool,

    #[arg(long, help = "Debug")]
    pub debug: bool,

    /// Render pages with headless Chromium (default: on).
    #[arg(long, default_value_t = true)]
    pub render: bool,

    #[arg(long, help = "Disable Chromium; HTTP-only")]
    pub no_render: bool,

    #[arg(
        long,
        value_parser = ["load", "domcontentloaded", "networkidle"],
        default_value = "domcontentloaded"
    )]
    pub render_wait: String,

    #[arg(long, default_value_t = 20, help = "Navigation timeout (seconds)")]
    pub render_timeout: u64,

    #[arg(long, help = "Path to Chrome/Chromium binary")]
    pub browser_path: Option<PathBuf>,

    /// Chrome user-data-dir (e.g. ~/Library/Application Support/Google/Chrome).
    /// Lets the browser reuse your existing cookies/session — useful for sites
    /// behind login or CAPTCHA.  Implies --headed unless overridden.
    #[arg(long, help = "Chrome user-data-dir for persistent profile")]
    pub browser_user_data_dir: Option<PathBuf>,

    /// Chrome profile directory inside user-data-dir (e.g. "Profile 1").
    #[arg(long, help = "Chrome profile directory name (e.g. 'Profile 1')")]
    pub browser_profile_dir: Option<String>,

    /// When set, pause before each navigation and wait for the human to press
    /// Enter. Useful for solving CAPTCHAs or completing logins manually.
    #[arg(
        long,
        help = "Pause before each page fetch — human solves CAPTCHA then presses Enter"
    )]
    pub human_in_loop: bool,

    /// How long (seconds) to wait for the human before timing out and moving on.
    #[arg(
        long,
        default_value_t = 300,
        help = "Seconds to wait for human input (--human-in-loop)"
    )]
    pub human_timeout: u64,

    #[arg(
        long,
        help = "Show browser window (required when --browser-user-data-dir is set)"
    )]
    pub headed: bool,

    #[arg(long, default_value_t = 4, help = "Max concurrent page fetches")]
    pub concurrency: usize,

    #[arg(long, help = "Do not fall back to static HTTP on browser failure")]
    pub no_fallback: bool,

    #[arg(long, help = "Disable TLS certificate verification (Ruby parity)")]
    pub insecure: bool,

    #[arg(long, help = "Pass --no-sandbox to Chromium (containers)")]
    pub no_sandbox: bool,

    #[arg(long, help = "Stop after this many pages (0 = unlimited)")]
    pub max_pages: Option<u64>,

    #[arg(value_name = "URL", required = true)]
    pub url: String,
}

pub(crate) fn finalize_spider_in_place(a: &mut CewlArgs) {
    if a.no_render {
        a.render = false;
    }
    if a.capture_url_structure {
        a.capture_paths = true;
        a.capture_subdomains = true;
        a.capture_domain = true;
    }
    if a.browser_user_data_dir.is_some() {
        a.headed = true;
    }
}

impl CewlArgs {
    pub fn parse_from_env() -> Result<Self, clap::Error> {
        let mut a = Self::try_parse()?;
        finalize_spider_in_place(&mut a);
        Ok(a)
    }

    /// True when the browser should run in headed (visible) mode.
    pub fn browser_headed(&self) -> bool {
        self.headed || self.human_in_loop || self.browser_user_data_dir.is_some()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.depth < 0 {
            return Err("--depth must be >= 0".into());
        }
        if self.min_word_length < 1 {
            return Err("--min_word_length must be >= 1".into());
        }
        if let Some(m) = self.max_word_length {
            if m < 1 {
                return Err("--max_word_length must be >= 1".into());
            }
        }
        if self.groups < -1 {
            return Err("--groups must be >= -1".into());
        }
        if self.auth_type.is_some() && (self.auth_user.is_none() || self.auth_pass.is_none()) {
            return Err("If using --auth_type you must provide --auth_user and --auth_pass".into());
        }
        if (self.auth_user.is_some() || self.auth_pass.is_some()) && self.auth_type.is_none() {
            return Err("Authentication details provided but no --auth_type".into());
        }
        if let Some(t) = &self.auth_type {
            let tl = t.to_lowercase();
            if tl != "basic" && tl != "digest" {
                return Err("Invalid --auth_type, use basic or digest".into());
            }
        }
        let meta_dir = &self.meta_temp_dir;
        if !meta_dir.is_dir() {
            return Err("--meta-temp-dir must be an existing directory".into());
        }
        let probe = meta_dir.join(".cewl_write_probe");
        std::fs::write(&probe, b"ok")
            .map_err(|_| "The meta temp directory is not writable".to_string())?;
        let _ = std::fs::remove_file(&probe);
        Ok(())
    }

    pub fn meta_temp_dir_str(&self) -> String {
        let mut s = self.meta_temp_dir.display().to_string();
        if !s.ends_with('/') {
            s.push('/');
        }
        s
    }

    pub fn proxy_port_or_default(&self) -> u16 {
        self.proxy_port.unwrap_or(8080)
    }

    pub fn wordlist_enabled(&self) -> bool {
        !self.no_words
    }
}

#[derive(Clone, Debug, Parser)]
#[command(
    name = "cewl",
    version = VERSION,
    about = "Files Already Bagged — extract author/creator metadata from local files (fab.rb parity)"
)]
pub struct FabArgs {
    #[arg(short = 'v', help = "Verbose")]
    pub verbose: bool,

    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

impl FabArgs {
    pub fn parse_from_env() -> Result<Self, clap::Error> {
        Self::try_parse()
    }
}
