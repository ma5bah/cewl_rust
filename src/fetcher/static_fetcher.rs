//! Static HTTP fetcher — reqwest with manual redirect handling (Ruby parity).

use crate::fetcher::FetchOutcome;
use digest_auth::{AuthContext, WwwAuthenticateHeader};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::redirect::Policy;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone)]
pub struct StaticConfig {
    pub user_agent: Option<String>,
    /// Parsed `name: value` header pairs.
    pub headers: Vec<(String, String)>,
    pub proxy_url: Option<String>,
    /// Accept invalid TLS certs (mirrors Ruby VERIFY_NONE).
    pub insecure: bool,
    pub auth_basic: Option<(String, String)>,
    pub auth_digest: Option<(String, String)>,
    pub timeout: Duration,
}

impl Default for StaticConfig {
    fn default() -> Self {
        Self {
            user_agent: None,
            headers: Vec::new(),
            proxy_url: None,
            insecure: true,
            auth_basic: None,
            auth_digest: None,
            timeout: Duration::from_secs(60),
        }
    }
}

pub struct StaticFetcher {
    client: reqwest::Client,
    cfg: StaticConfig,
}

impl StaticFetcher {
    pub fn new(cfg: StaticConfig) -> Result<Self, reqwest::Error> {
        let mut b = reqwest::Client::builder()
            .redirect(Policy::none())
            .danger_accept_invalid_certs(cfg.insecure)
            .timeout(cfg.timeout);
        if let Some(ref p) = cfg.proxy_url {
            b = b.proxy(reqwest::Proxy::all(p)?);
        }
        let client = b.build()?;
        Ok(Self { client, cfg })
    }

    fn build_headers(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in &self.cfg.headers {
            if let (Ok(name), Ok(val)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
                map.insert(name, val);
            }
        }
        if let Some(ref ua) = self.cfg.user_agent {
            if let Ok(val) = HeaderValue::from_str(ua) {
                map.insert(reqwest::header::USER_AGENT, val);
            }
        }
        map
    }

    /// Fetch `url`, returning one step of the redirect chain so the crawler
    /// can decide whether to follow (matches Ruby `get_page` manual redirect).
    pub async fn fetch(&self, url: &str) -> FetchOutcome {
        let mut headers = self.build_headers();

        let mut req = self.client.get(url).headers(headers.clone());
        if let Some((ref u, ref p)) = self.cfg.auth_basic {
            req = req.basic_auth(u, Some(p));
        }

        let res = match req.send().await {
            Ok(r) => r,
            Err(e) => return FetchOutcome::Error(e.to_string()),
        };

        let status = res.status();

        // --- digest auth ---------------------------------------------------
        if status == reqwest::StatusCode::UNAUTHORIZED {
            if let Some((ref user, ref pass)) = self.cfg.auth_digest {
                if let Some(www_raw) = res
                    .headers()
                    .get("www-authenticate")
                    .and_then(|h| h.to_str().ok())
                {
                    if let Ok(mut prompt) = WwwAuthenticateHeader::parse(www_raw) {
                        let uri = url::Url::parse(url)
                            .map(|u| u.path().to_string())
                            .unwrap_or_else(|_| url.to_string());
                        let ctx = AuthContext::new(user.as_str(), pass.as_str(), uri.as_str());
                        if let Ok(answer) = prompt.respond(&ctx) {
                            let auth_val = answer.to_string();
                            if let Ok(val) = HeaderValue::from_str(&auth_val) {
                                headers.insert(HeaderName::from_static("authorization"), val);
                            }
                            let res2 = match self.client.get(url).headers(headers).send().await {
                                Ok(r) => r,
                                Err(e) => return FetchOutcome::Error(e.to_string()),
                            };
                            return self.map_response(res2, url).await;
                        }
                    }
                }
            }
            return FetchOutcome::Unauthorized;
        }

        // --- redirect ------------------------------------------------------
        if status.is_redirection() {
            if let Some(loc) = res
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
            {
                let next = url::Url::parse(url)
                    .ok()
                    .and_then(|b| b.join(loc).ok())
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| loc.to_string());
                return FetchOutcome::Redirect {
                    from: url.to_string(),
                    to: next,
                };
            }
        }

        self.map_response(res, url).await
    }

    async fn map_response(&self, res: reqwest::Response, url: &str) -> FetchOutcome {
        let status = res.status();
        let ct = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());
        let body = res.bytes().await.unwrap_or_default().to_vec();
        FetchOutcome::Response {
            final_url: url.to_string(),
            status,
            content_type: ct,
            body,
        }
    }
}
