use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    pub sites: Vec<SiteEntry>,
    pub crawl: CrawlConfig,
    pub logs: LogsConfig,
    pub ban: BanConfig,
    pub web: WebConfig,
    pub ssh: SshConfig,
    pub mail: MailConfig,
    pub nft: NftConfig,
    pub whitelist: WhitelistConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteEntry {
    pub base_url: String,
    pub log_path: PathBuf,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub extra_paths: Vec<String>,
}

impl SiteEntry {
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(self.base_url.as_str())
    }

    pub fn crawl_config(&self, crawl: &CrawlConfig) -> SiteConfig {
        SiteConfig {
            base_url: self.base_url.clone(),
            crawl_timeout_secs: crawl.timeout_secs,
            max_pages: crawl.max_pages,
            max_depth: crawl.max_depth,
            extra_paths: self.extra_paths.clone(),
            user_agent: crawl.user_agent.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    pub base_url: String,
    pub crawl_timeout_secs: u64,
    pub max_pages: usize,
    pub max_depth: u32,
    pub extra_paths: Vec<String>,
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrawlConfig {
    #[serde(default = "default_crawl_interval_secs")]
    pub interval_secs: u64,
    #[serde(default = "default_crawl_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogsConfig {
    pub format: LogFormat,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Caddy,
    Nginx,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebProbesConfig {
    #[serde(default = "default_web_probes_enabled")]
    pub enabled: bool,
    #[serde(default = "default_probe_paths")]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebAuthConfig {
    #[serde(default = "default_web_auth_enabled")]
    pub enabled: bool,
    #[serde(default = "default_auth_prefixes")]
    pub prefixes: Vec<String>,
    #[serde(default = "default_threshold")]
    pub threshold: u32,
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct WebConfig {
    #[serde(default)]
    pub probes: WebProbesConfig,
    #[serde(default)]
    pub auth: WebAuthConfig,
}

impl Default for WebProbesConfig {
    fn default() -> Self {
        Self {
            enabled: default_web_probes_enabled(),
            paths: default_probe_paths(),
        }
    }
}

impl Default for WebAuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_web_auth_enabled(),
            prefixes: default_auth_prefixes(),
            threshold: default_threshold(),
            window_secs: default_window_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailConfig {
    #[serde(default = "default_mail_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub units: Vec<String>,
    #[serde(default = "default_mail_threshold")]
    pub threshold: u32,
    #[serde(default = "default_mail_window_secs")]
    pub window_secs: u64,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            enabled: default_mail_enabled(),
            units: Vec::new(),
            threshold: default_mail_threshold(),
            window_secs: default_mail_window_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SshConfig {
    #[serde(default = "default_ssh_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ssh_unit")]
    pub unit: String,
    #[serde(default = "default_ssh_threshold")]
    pub threshold: u32,
    #[serde(default = "default_ssh_window_secs")]
    pub window_secs: u64,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            enabled: default_ssh_enabled(),
            unit: default_ssh_unit(),
            threshold: default_ssh_threshold(),
            window_secs: default_ssh_window_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BanConfig {
    #[serde(default = "default_threshold")]
    pub threshold: u32,
    #[serde(default = "default_window_secs")]
    pub window_secs: u64,
    #[serde(default = "default_ban_duration_secs")]
    pub ban_duration_secs: u64,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NftConfig {
    pub table: String,
    pub set: String,
    #[serde(default = "default_nft_timeout")]
    pub timeout: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WhitelistConfig {
    #[serde(default)]
    pub ips: Vec<IpAddr>,
    #[serde(default)]
    pub cidrs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    sites: Vec<SiteEntry>,
    site: Option<LegacySite>,
    crawl: Option<CrawlConfig>,
    logs: RawLogsConfig,
    ban: BanConfig,
    #[serde(default)]
    web: WebConfig,
    #[serde(default)]
    ssh: SshConfig,
    #[serde(default)]
    mail: MailConfig,
    nft: NftConfig,
    #[serde(default)]
    whitelist: WhitelistConfig,
}

#[derive(Debug, Deserialize)]
struct LegacySite {
    pub base_url: String,
    #[serde(default = "default_crawl_interval_secs")]
    crawl_interval_secs: u64,
    #[serde(default = "default_crawl_timeout_secs")]
    crawl_timeout_secs: u64,
    #[serde(default = "default_max_pages")]
    max_pages: usize,
    #[serde(default = "default_max_depth")]
    max_depth: u32,
    #[serde(default)]
    extra_paths: Vec<String>,
    #[serde(default = "default_user_agent")]
    user_agent: String,
}

#[derive(Debug, Deserialize)]
struct RawLogsConfig {
    pub format: LogFormat,
    path: Option<PathBuf>,
    #[serde(default = "default_poll_interval_ms")]
    poll_interval_ms: u64,
}

impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file {}", path.display()))?;
        let raw: RawConfig = toml::from_str(&raw).context("failed to parse config TOML")?;
        raw.into_config()
    }

    pub fn crawl_interval(&self) -> Duration {
        Duration::from_secs(self.crawl.interval_secs)
    }

    pub fn log_poll_interval(&self) -> Duration {
        Duration::from_millis(self.logs.poll_interval_ms)
    }

    pub fn ban_window(&self) -> Duration {
        Duration::from_secs(self.ban.window_secs)
    }
}

impl RawConfig {
    fn into_config(self) -> Result<Config> {
        let logs = LogsConfig {
            format: self.logs.format,
            poll_interval_ms: self.logs.poll_interval_ms,
        };

        let crawl = self.crawl.unwrap_or_else(|| CrawlConfig {
            interval_secs: self
                .site
                .as_ref()
                .map(|s| s.crawl_interval_secs)
                .unwrap_or_else(default_crawl_interval_secs),
            timeout_secs: self
                .site
                .as_ref()
                .map(|s| s.crawl_timeout_secs)
                .unwrap_or_else(default_crawl_timeout_secs),
            max_pages: self
                .site
                .as_ref()
                .map(|s| s.max_pages)
                .unwrap_or_else(default_max_pages),
            max_depth: self
                .site
                .as_ref()
                .map(|s| s.max_depth)
                .unwrap_or_else(default_max_depth),
            user_agent: self
                .site
                .as_ref()
                .map(|s| s.user_agent.clone())
                .unwrap_or_else(default_user_agent),
        });

        let sites = if !self.sites.is_empty() {
            self.sites
        } else if let Some(site) = self.site {
            let log_path = self
                .logs
                .path
                .context("legacy config requires [logs].path or [[sites]] entries")?;
            vec![SiteEntry {
                base_url: site.base_url,
                log_path,
                name: None,
                extra_paths: site.extra_paths,
            }]
        } else {
            bail!("config must define [[sites]] or legacy [site] section");
        };

        Ok(Config {
            sites,
            crawl,
            logs,
            ban: self.ban,
            web: self.web,
            ssh: self.ssh,
            mail: self.mail,
            nft: self.nft,
            whitelist: self.whitelist,
        })
    }
}

fn default_crawl_interval_secs() -> u64 {
    86_400
}

fn default_crawl_timeout_secs() -> u64 {
    10
}

fn default_max_pages() -> usize {
    500
}

fn default_max_depth() -> u32 {
    5
}

fn default_user_agent() -> String {
    "Bouncer/1.0 (+https://github.com/Normal66/Bouncer)".into()
}

fn default_poll_interval_ms() -> u64 {
    500
}

fn default_threshold() -> u32 {
    5
}

fn default_window_secs() -> u64 {
    60
}

fn default_ban_duration_secs() -> u64 {
    3_600
}

fn default_dry_run() -> bool {
    false
}

fn default_web_probes_enabled() -> bool {
    true
}

fn default_web_auth_enabled() -> bool {
    true
}

fn default_probe_paths() -> Vec<String> {
    [
        "/.env",
        "/.git/config",
        "/.git/HEAD",
        "/.aws/credentials",
        "/wp-login.php",
        "/wp-admin",
        "/xmlrpc.php",
        "/phpmyadmin",
        "/pma",
        "/admin/config.php",
        "/vendor/phpunit/phpunit/src/Util/PHP/eval-stdin.php",
        "/config.json",
        "/actuator",
        "/server-status",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn default_auth_prefixes() -> Vec<String> {
    [
        "/admin",
        "/login",
        "/wp-login.php",
        "/wp-admin",
        "/api/login",
        "/api/auth",
        "/user/login",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn default_mail_enabled() -> bool {
    false
}

fn default_mail_threshold() -> u32 {
    3
}

fn default_mail_window_secs() -> u64 {
    120
}

fn default_ssh_enabled() -> bool {
    false
}

fn default_ssh_unit() -> String {
    "ssh".into()
}

fn default_ssh_threshold() -> u32 {
    3
}

fn default_ssh_window_secs() -> u64 {
    120
}

fn default_nft_timeout() -> String {
    "1h".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_config() {
        let raw = r#"
            [site]
            base_url = "https://example.com"

            [logs]
            path = "/var/log/caddy/access.log"
            format = "caddy"

            [ban]
            threshold = 3
            window_secs = 30

            [nft]
            table = "inet filter"
            set = "blocked_ips"
        "#;

        let raw: RawConfig = toml::from_str(raw).expect("config should parse");
        let cfg = raw.into_config().expect("legacy config");
        assert_eq!(cfg.sites.len(), 1);
        assert_eq!(cfg.logs.format, LogFormat::Caddy);
        assert_eq!(cfg.ban.threshold, 3);
    }

    #[test]
    fn parses_web_and_mail_config() {
        let raw = r#"
            [logs]
            format = "nginx"

            [[sites]]
            base_url = "https://example.com"
            log_path = "/var/log/nginx/access.log"

            [web.probes]
            enabled = true
            paths = ["/.env"]

            [web.auth]
            enabled = true
            prefixes = ["/admin"]
            threshold = 2

            [mail]
            enabled = true
            units = ["postfix"]

            [ban]

            [nft]
            table = "inet filter"
            set = "blocked_ips"
        "#;

        let raw: RawConfig = toml::from_str(raw).expect("config should parse");
        let cfg = raw.into_config().expect("web/mail config");
        assert!(cfg.web.probes.enabled);
        assert_eq!(cfg.web.probes.paths, vec!["/.env"]);
        assert_eq!(cfg.web.auth.prefixes, vec!["/admin"]);
        assert_eq!(cfg.web.auth.threshold, 2);
        assert!(cfg.mail.enabled);
        assert_eq!(cfg.mail.units, vec!["postfix"]);
    }

    #[test]
    fn parses_multi_site_config() {
        let raw = r#"
            [crawl]
            interval_secs = 3600

            [logs]
            format = "caddy"

            [[sites]]
            name = "main"
            base_url = "https://example.com"
            log_path = "/var/log/caddy/a.log"

            [[sites]]
            base_url = "https://other.example"
            log_path = "/var/log/caddy/b.log"

            [ban]

            [nft]
            table = "inet filter"
            set = "blocked_ips"
        "#;

        let raw: RawConfig = toml::from_str(raw).expect("config should parse");
        let cfg = raw.into_config().expect("multi-site config");
        assert_eq!(cfg.sites.len(), 2);
        assert_eq!(cfg.crawl.interval_secs, 3600);
    }
}
