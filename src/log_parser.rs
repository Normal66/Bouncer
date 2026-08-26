use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

use crate::config::LogFormat;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessEntry {
    pub ip: String,
    pub path: String,
    pub status: u16,
}

static NGINX_COMBINED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^(?P<ip>\S+)\s+\S+\s+\S+\s+\[[^\]]+\]\s+"(?:GET|HEAD|POST|PUT|DELETE|PATCH|OPTIONS|CONNECT|TRACE)\s+(?P<path>\S+)\s+[^"]+"\s+(?P<status>\d{3})\s+"#,
    )
    .expect("nginx combined regex")
});

#[derive(Debug, Deserialize)]
struct CaddyJsonLog {
    request: CaddyRequest,
    status: u16,
    #[serde(default, alias = "client_ip")]
    remote_ip: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaddyRequest {
    uri: String,
    remote_ip: Option<String>,
}

pub fn parse_line(format: LogFormat, line: &str) -> Result<Option<AccessEntry>> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    match format {
        LogFormat::Caddy => parse_caddy(line),
        LogFormat::Nginx => parse_nginx(line),
    }
}

fn parse_caddy(line: &str) -> Result<Option<AccessEntry>> {
    let record: CaddyJsonLog = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let ip = record
        .remote_ip
        .or(record.request.remote_ip)
        .filter(|value| !value.is_empty())
        .context("caddy log line missing remote IP")?;

    let path = extract_path(&record.request.uri);

    Ok(Some(AccessEntry {
        ip,
        path,
        status: record.status,
    }))
}

fn parse_nginx(line: &str) -> Result<Option<AccessEntry>> {
    let caps = match NGINX_COMBINED.captures(line) {
        Some(value) => value,
        None => return Ok(None),
    };

    let ip = caps
        .name("ip")
        .map(|m| m.as_str().to_owned())
        .context("nginx log line missing IP")?;
    let raw_path = caps
        .name("path")
        .map(|m| m.as_str())
        .context("nginx log line missing path")?;
    let status = caps
        .name("status")
        .map(|m| m.as_str())
        .context("nginx log line missing status")?
        .parse::<u16>()
        .context("invalid nginx status code")?;

    Ok(Some(AccessEntry {
        ip,
        path: extract_path(raw_path),
        status,
    }))
}

fn extract_path(uri: &str) -> String {
    let path = uri.split('?').next().unwrap_or(uri);
    crate::crawler::normalize_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_caddy_json() {
        let line = r#"{"request":{"uri":"/missing","remote_ip":"203.0.113.10"},"status":404}"#;
        let entry = parse_line(LogFormat::Caddy, line)
            .unwrap()
            .expect("entry");
        assert_eq!(entry.ip, "203.0.113.10");
        assert_eq!(entry.path, "/missing");
        assert_eq!(entry.status, 404);
    }

    #[test]
    fn parses_nginx_combined() {
        let line = r#"203.0.113.10 - - [26/Aug/2026:10:00:00 +0000] "GET /admin HTTP/1.1" 404 123 "-" "curl/8.0""#;
        let entry = parse_line(LogFormat::Nginx, line)
            .unwrap()
            .expect("entry");
        assert_eq!(entry.ip, "203.0.113.10");
        assert_eq!(entry.path, "/admin");
        assert_eq!(entry.status, 404);
    }
}
