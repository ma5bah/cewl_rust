//! URL-structure word captures: paths, subdomains, registrable domain.

use regex::Regex;
use std::sync::LazyLock;

static PATH_CLEAN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-zA-Z0-9\-_]").unwrap());

pub fn extract_path_components(url: &str) -> Vec<String> {
    let Ok(parsed) = url::Url::parse(url) else {
        return vec![];
    };
    let path = parsed.path();
    if path.is_empty() || path == "/" {
        return vec![];
    }
    let mut out = Vec::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        let mut clean = part.split('?').next().unwrap_or(part).to_string();
        if let Some(i) = clean.rfind('.') {
            clean.truncate(i);
        }
        clean = PATH_CLEAN.replace_all(&clean, "").to_string();
        if clean.len() >= 3 {
            out.push(clean);
        }
    }
    out
}

pub fn extract_subdomain_components(url: &str) -> Vec<String> {
    let Ok(parsed) = url::Url::parse(url) else {
        return vec![];
    };
    let Some(host) = parsed.host_str() else {
        return vec![];
    };
    let host_parts: Vec<&str> = host.split('.').collect();

    // Use psl to find registrable domain width
    let domain_width = match psl::domain(host.as_bytes()) {
        Some(d) => {
            let d_str = std::str::from_utf8(d.as_bytes()).unwrap_or(host);
            d_str.split('.').count()
        }
        None => {
            // Fallback: assume last two labels are the domain
            2.min(host_parts.len())
        }
    };

    let subdomain_count = host_parts.len().saturating_sub(domain_width);
    host_parts[..subdomain_count]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn extract_registrable_domain(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    if let Some(d) = psl::domain(host.as_bytes()) {
        return std::str::from_utf8(d.as_bytes())
            .ok()
            .map(|s| s.to_string());
    }
    // Fallback
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() >= 2 {
        return Some(parts[parts.len() - 2..].join("."));
    }
    None
}
