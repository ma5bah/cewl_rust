//! Headless Chrome fetcher via chromiumoxide (CDP).
//!
//! Key features:
//! - Persistent Chrome profile (--browser-user-data-dir / --browser-profile-dir)
//!   so cookies/sessions from a real Chrome install are reused.
//! - Human-in-loop (--human-in-loop): pauses before each navigation so a human
//!   can solve CAPTCHAs or complete logins, then resumes on Enter.
//! - Network sniffing: every URL Chrome requests is fed back as a discovered link.

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::EventRequestWillBeSent;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::fetcher::FetchOutcome;

pub struct BrowserCfg {
    pub browser_path: Option<PathBuf>,
    pub user_data_dir: Option<PathBuf>,
    pub profile_directory: Option<String>,
    pub headed: bool,
    pub no_sandbox: bool,
    pub user_agent: Option<String>,
    pub proxy: Option<String>,
    pub insecure: bool,
    pub render_wait: RenderWait,
    pub render_timeout: Duration,
    pub human_in_loop: bool,
    pub human_timeout: Duration,
    pub concurrency: usize,
    pub extra_headers: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug)]
pub enum RenderWait {
    Load,
    DomContentLoaded,
    NetworkIdle,
}

impl RenderWait {
    pub fn parse(s: &str) -> Self {
        match s {
            "load" => Self::Load,
            "networkidle" => Self::NetworkIdle,
            _ => Self::DomContentLoaded,
        }
    }
}

pub struct BrowserFetcher {
    browser: Arc<Browser>,
    cfg: Arc<BrowserCfg>,
    sem: Arc<Semaphore>,
}

impl BrowserFetcher {
    pub async fn launch(cfg: BrowserCfg) -> Result<Self, anyhow::Error> {
        let mut builder = BrowserConfig::builder();

        if cfg.headed || cfg.user_data_dir.is_some() {
            builder = builder.with_head();
        }

        if let Some(ref p) = cfg.browser_path {
            builder = builder.chrome_executable(p);
        }

        // Build launch args
        let mut args: Vec<String> = Vec::new();
        if cfg.no_sandbox {
            args.push("--no-sandbox".into());
        }
        if cfg.insecure {
            args.push("--ignore-certificate-errors".into());
        }
        if let Some(ref p) = cfg.proxy {
            args.push(format!("--proxy-server={p}"));
        }
        if let Some(ref ua) = cfg.user_agent {
            args.push(format!("--user-agent={ua}"));
        }
        if let Some(ref udd) = cfg.user_data_dir {
            args.push(format!("--user-data-dir={}", udd.display()));
        }
        if let Some(ref pd) = cfg.profile_directory {
            args.push(format!("--profile-directory={pd}"));
        }
        for a in args {
            builder = builder.arg(a);
        }

        let config = builder.build().map_err(|e| anyhow::anyhow!("{e}"))?;
        let (browser, mut handler) = Browser::launch(config).await?;

        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let sem = Arc::new(Semaphore::new(cfg.concurrency.max(1)));
        Ok(Self {
            browser: Arc::new(browser),
            cfg: Arc::new(cfg),
            sem,
        })
    }

    pub async fn fetch(&self, url: &str, verbose: bool) -> FetchOutcome {
        let _permit = match self.sem.acquire().await {
            Ok(p) => p,
            Err(_) => return FetchOutcome::Error("semaphore closed".into()),
        };

        let page = match self.browser.new_page("about:blank").await {
            Ok(p) => p,
            Err(e) => return FetchOutcome::Error(format!("new page: {e}")),
        };

        // Set UA override via CDP if requested
        if let Some(ref ua) = self.cfg.user_agent {
            let _ = page.set_user_agent(ua.clone()).await;
        }

        // Sniff outbound request URLs via CDP events
        let sniffed: Arc<tokio::sync::Mutex<Vec<String>>> =
            Arc::new(tokio::sync::Mutex::new(Vec::new()));

        if let Ok(mut events) = page.event_listener::<EventRequestWillBeSent>().await {
            let sniffed2 = sniffed.clone();
            tokio::spawn(async move {
                while let Some(evt) = events.next().await {
                    sniffed2.lock().await.push(evt.request.url.clone());
                }
            });
        }

        // Human-in-loop: pause and wait for Enter before navigating
        if self.cfg.human_in_loop {
            eprintln!("\n[human-in-loop] Ready to navigate: {url}");
            eprintln!("  Complete any login/CAPTCHA in the browser, then press Enter...");
            let _ = timeout(
                self.cfg.human_timeout,
                tokio::task::spawn_blocking(|| {
                    let mut s = String::new();
                    let _ = std::io::stdin().read_line(&mut s);
                }),
            )
            .await;
        }

        // Navigate
        let nav = timeout(self.cfg.render_timeout, page.goto(url)).await;
        match nav {
            Err(_) => {
                if verbose {
                    eprintln!("[browser] timeout: {url}");
                }
                return FetchOutcome::Error(format!("timeout: {url}"));
            }
            Ok(Err(e)) => {
                if verbose {
                    eprintln!("[browser] nav error: {e}");
                }
                return FetchOutcome::Error(format!("nav: {e}"));
            }
            Ok(Ok(_)) => {}
        }

        // Wait for networkidle if requested
        if matches!(self.cfg.render_wait, RenderWait::NetworkIdle) {
            let _ = timeout(self.cfg.render_timeout, page.wait_for_navigation()).await;
        }

        // Get rendered HTML
        let html = match timeout(Duration::from_secs(15), page.content()).await {
            Ok(Ok(h)) => h,
            _ => return FetchOutcome::Error(format!("content() failed: {url}")),
        };

        let final_url = page
            .url()
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| url.to_string());
        let discovered = sniffed.lock().await.clone();

        // Close page to free resources
        let _ = page.close().await;

        FetchOutcome::BrowserResponse {
            final_url,
            status: reqwest::StatusCode::OK,
            content_type: Some("text/html; charset=utf-8".into()),
            body: html.into_bytes(),
            discovered_urls: discovered,
        }
    }
}
