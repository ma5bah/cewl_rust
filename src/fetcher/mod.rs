//! Fetch outcomes, trait, and dispatcher.

pub mod browser_fetcher;
pub mod static_fetcher;

use reqwest::StatusCode;

#[derive(Debug)]
pub enum FetchOutcome {
    /// Normal HTML/text response (static path).
    Response {
        final_url: String,
        status: StatusCode,
        content_type: Option<String>,
        body: Vec<u8>,
    },
    /// Browser response, includes URLs sniffed from network events.
    BrowserResponse {
        final_url: String,
        status: StatusCode,
        content_type: Option<String>,
        body: Vec<u8>,
        discovered_urls: Vec<String>,
    },
    /// 3xx — caller must re-enqueue `to`.
    Redirect { from: String, to: String },
    /// 401 with no viable auth.
    Unauthorized,
    /// Any other error (socket, parse, timeout …).
    Error(String),
}

impl FetchOutcome {
    pub fn is_htmlish(&self) -> bool {
        let ct = match self {
            FetchOutcome::Response { content_type, .. } => content_type,
            FetchOutcome::BrowserResponse { content_type, .. } => content_type,
            _ => return false,
        };
        let Some(ct) = ct else { return true };
        let lower = ct.to_ascii_lowercase();
        lower.contains("text/html")
            || lower.contains("application/xhtml")
            || lower.contains("text/plain")
    }

    pub fn is_binary_content_type(&self) -> bool {
        let ct = match self {
            FetchOutcome::Response { content_type, .. } => content_type,
            FetchOutcome::BrowserResponse { content_type, .. } => content_type,
            _ => return false,
        };
        let Some(ct) = ct else { return false };
        let lower = ct.to_ascii_lowercase();
        lower.contains("pdf")
            || lower.contains("msword")
            || lower.contains("officedocument")
            || lower.contains("octet-stream")
            || lower.contains("zip")
    }

    pub fn body_and_url(self) -> Option<(String, Vec<u8>, Option<String>)> {
        match self {
            FetchOutcome::Response {
                final_url,
                body,
                content_type,
                ..
            } => Some((final_url, body, content_type)),
            FetchOutcome::BrowserResponse {
                final_url,
                body,
                content_type,
                ..
            } => Some((final_url, body, content_type)),
            _ => None,
        }
    }

    pub fn discovered_urls(&self) -> &[String] {
        match self {
            FetchOutcome::BrowserResponse {
                discovered_urls, ..
            } => discovered_urls,
            _ => &[],
        }
    }
}
