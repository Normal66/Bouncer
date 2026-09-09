use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use regex::Regex;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use url::Url;

use crate::config::SiteConfig;

#[derive(Clone)]
pub struct PathCatalog {
    inner: Arc<RwLock<HashSet<String>>>,
}

impl PathCatalog {
    pub fn new(initial: HashSet<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(initial)),
        }
    }

    pub async fn contains(&self, path: &str) -> bool {
        self.inner.read().await.contains(path)
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    pub async fn replace(&self, paths: HashSet<String>) {
        let count = paths.len();
        *self.inner.write().await = paths;
        info!(count, "path catalog updated");
    }
}

pub async fn crawl_site(config: &SiteConfig) -> Result<HashSet<String>> {
    let base = Url::parse(&config.base_url).context("invalid site.base_url")?;
    let host = base
        .host_str()
        .context("site.base_url must include a host")?
        .to_owned();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.crawl_timeout_secs))
        .user_agent(&config.user_agent)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .context("failed to build HTTP client")?;

    let href_re = Regex::new(r#"(?i)href\s*=\s*["']([^"'#]+)"#).context("href regex")?;

    let mut discovered = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();

    let start_path = normalize_path(base.path());
    discovered.insert(start_path.clone());
    queue.push_back((start_path, 0));

    while let Some((path, depth)) = queue.pop_front() {
        if discovered.len() >= config.max_pages {
            warn!(max_pages = config.max_pages, "crawl page limit reached");
            break;
        }
        if depth > config.max_depth {
            continue;
        }

        let page_url = base
            .join(&path)
            .with_context(|| format!("join base with {path}"))?;
        debug!(%page_url, depth, "crawling page");

        let response = match client.get(page_url.clone()).send().await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(%page_url, error = %err, "crawl request failed");
                continue;
            }
        };

        if !response.status().is_success() {
            warn!(%page_url, status = %response.status(), "non-success status while crawling");
            continue;
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/html") && !content_type.is_empty() {
            continue;
        }

        let body = match response.text().await {
            Ok(text) => text,
            Err(err) => {
                warn!(%page_url, error = %err, "failed to read response body");
                continue;
            }
        };

        for capture in href_re.captures_iter(&body) {
            let href = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
            if let Some(path) = resolve_href(&base, &host, href)
                && discovered.insert(path.clone())
                && discovered.len() < config.max_pages
            {
                queue.push_back((path, depth + 1));
            }
        }
    }

    for extra in &config.extra_paths {
        discovered.insert(normalize_path(extra));
    }

    info!(pages = discovered.len(), "crawl finished");
    Ok(discovered)
}

pub fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().to_string();
    if normalized.is_empty() {
        normalized.push('/');
    }
    if !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn resolve_href(base: &Url, host: &str, href: &str) -> Option<String> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("tel:")
    {
        return None;
    }

    let resolved = base.join(trimmed).ok()?;
    if resolved.scheme() != "http" && resolved.scheme() != "https" {
        return None;
    }
    if resolved.host_str()? != host {
        return None;
    }

    Some(normalize_path(resolved.path()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_paths() {
        assert_eq!(normalize_path(""), "/");
        assert_eq!(normalize_path("about"), "/about");
        assert_eq!(normalize_path("/about/"), "/about");
    }

    #[test]
    fn resolve_relative_href() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        assert_eq!(
            resolve_href(&base, "example.com", "../contact"),
            Some("/contact".into())
        );
        assert_eq!(
            resolve_href(&base, "example.com", "https://other.example/page"),
            None
        );
    }
}
