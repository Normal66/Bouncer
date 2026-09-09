use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::config::{BanConfig, MailConfig, SshConfig, WebConfig, WhitelistConfig};
use crate::crawler::PathCatalog;
use crate::log_parser::AccessEntry;
use crate::mail_parser::MailEvent;
use crate::nft::NftBanClient;
use crate::ssh_parser::SshEvent;
use crate::web_rules::matches_any_path;

#[derive(Clone)]
pub struct BanService {
    config: BanConfig,
    web: WebConfig,
    ssh_config: SshConfig,
    mail_config: MailConfig,
    whitelist: WhitelistConfig,
    nft: NftBanClient,
    offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    auth_offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    ssh_offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    mail_offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl BanService {
    pub fn new(
        config: BanConfig,
        web: WebConfig,
        ssh_config: SshConfig,
        mail_config: MailConfig,
        whitelist: WhitelistConfig,
        nft: NftBanClient,
    ) -> Self {
        Self {
            config,
            web,
            ssh_config,
            mail_config,
            whitelist,
            nft,
            offenders: Arc::new(Mutex::new(HashMap::new())),
            auth_offenders: Arc::new(Mutex::new(HashMap::new())),
            ssh_offenders: Arc::new(Mutex::new(HashMap::new())),
            mail_offenders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_entry(
        &self,
        site: &str,
        entry: AccessEntry,
        catalog: &PathCatalog,
    ) -> Result<()> {
        if self.is_whitelisted(&entry.ip) {
            return Ok(());
        }

        if self.web.probes.enabled && matches_any_path(&entry.path, &self.web.probes.paths) {
            return self
                .ban_instant(
                    &entry.ip,
                    site,
                    &format!("probe path {}", entry.path),
                    "IP banned for probe path",
                )
                .await;
        }

        if self.web.auth.enabled
            && matches!(entry.status, 401 | 403)
            && matches_any_path(&entry.path, &self.web.auth.prefixes)
        {
            let count =
                record_hit(&self.auth_offenders, &entry.ip, self.web.auth.window_secs).await;
            if count < self.web.auth.threshold {
                return Ok(());
            }

            self.auth_offenders.lock().await.remove(&entry.ip);
            return self
                .ban_with_dry_run(
                    &entry.ip,
                    site,
                    count,
                    &format!("auth failure on {}", entry.path),
                )
                .await;
        }

        if entry.status != 404 {
            return Ok(());
        }

        if catalog.contains(&entry.path).await {
            return Ok(());
        }

        let count = record_hit(&self.offenders, &entry.ip, self.config.window_secs).await;
        if count < self.config.threshold {
            return Ok(());
        }

        self.offenders.lock().await.remove(&entry.ip);
        self.ban_with_dry_run(
            &entry.ip,
            site,
            count,
            &format!("unknown path {}", entry.path),
        )
        .await
    }

    pub async fn handle_ssh_event(&self, event: SshEvent) -> Result<()> {
        if !self.ssh_config.enabled {
            return Ok(());
        }

        match event {
            SshEvent::InvalidUser { ip } => {
                self.ban_instant(
                    &ip,
                    "ssh",
                    "invalid SSH user",
                    "IP banned for invalid SSH user",
                )
                .await
            }
            SshEvent::FailedPassword { ip } => {
                self.ban_threshold(
                    &self.ssh_offenders,
                    &ip,
                    "ssh",
                    self.ssh_config.threshold,
                    self.ssh_config.window_secs,
                    "SSH brute force",
                )
                .await
            }
        }
    }

    pub async fn handle_mail_event(&self, unit: &str, event: MailEvent) -> Result<()> {
        if !self.mail_config.enabled {
            return Ok(());
        }

        match event {
            MailEvent::InvalidUser { ip } => {
                self.ban_instant(
                    &ip,
                    unit,
                    "invalid mail user",
                    "IP banned for invalid mail user",
                )
                .await
            }
            MailEvent::AuthFailed { ip } => {
                self.ban_threshold(
                    &self.mail_offenders,
                    &ip,
                    unit,
                    self.mail_config.threshold,
                    self.mail_config.window_secs,
                    "mail auth failure",
                )
                .await
            }
        }
    }

    async fn ban_instant(&self, ip: &str, source: &str, detail: &str, message: &str) -> Result<()> {
        if self.is_whitelisted(ip) {
            return Ok(());
        }

        if self.config.dry_run {
            warn!(source, ip = %ip, detail, "dry-run: would ban IP instantly");
            return Ok(());
        }

        self.nft.ban_ip(ip).await?;
        info!(source, ip = %ip, detail, "{message}");
        Ok(())
    }

    async fn ban_threshold(
        &self,
        offenders: &Mutex<HashMap<String, VecDeque<Instant>>>,
        ip: &str,
        source: &str,
        threshold: u32,
        window_secs: u64,
        reason: &str,
    ) -> Result<()> {
        if self.is_whitelisted(ip) {
            return Ok(());
        }

        let count = record_hit(offenders, ip, window_secs).await;
        if count < threshold {
            return Ok(());
        }

        offenders.lock().await.remove(ip);
        self.ban_with_dry_run(ip, source, count, reason).await
    }

    async fn ban_with_dry_run(
        &self,
        ip: &str,
        source: &str,
        count: u32,
        reason: &str,
    ) -> Result<()> {
        if self.config.dry_run {
            warn!(source, ip = %ip, count, reason, "dry-run: would ban IP");
            return Ok(());
        }

        self.nft.ban_ip(ip).await?;
        info!(source, ip = %ip, hits = count, reason, "IP banned");
        Ok(())
    }

    fn is_whitelisted(&self, ip: &str) -> bool {
        let parsed = match ip.parse::<IpAddr>() {
            Ok(value) => value,
            Err(_) => return false,
        };

        if self.whitelist.ips.contains(&parsed) {
            return true;
        }

        for cidr in &self.whitelist.cidrs {
            if ip_in_cidr(ip, cidr) {
                return true;
            }
        }

        false
    }
}

async fn record_hit(
    offenders: &Mutex<HashMap<String, VecDeque<Instant>>>,
    ip: &str,
    window_secs: u64,
) -> u32 {
    let now = Instant::now();
    let window = Duration::from_secs(window_secs);
    let mut offenders = offenders.lock().await;
    let hits = offenders.entry(ip.to_owned()).or_default();
    hits.push_back(now);
    while let Some(front) = hits.front() {
        if now.duration_since(*front) > window {
            hits.pop_front();
        } else {
            break;
        }
    }
    hits.len() as u32
}

fn ip_in_cidr(ip: &str, cidr: &str) -> bool {
    let (network, prefix) = match cidr.split_once('/') {
        Some(value) => value,
        None => return false,
    };

    let ip: IpAddr = match ip.parse() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let network: IpAddr = match network.parse() {
        Ok(value) => value,
        Err(_) => return false,
    };
    let prefix: u8 = match prefix.parse() {
        Ok(value) => value,
        Err(_) => return false,
    };

    match (ip, network) {
        (IpAddr::V4(ip), IpAddr::V4(net)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix as u32)
            };
            (u32::from(ip) & mask) == (u32::from(net) & mask)
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix as u32)
            };
            (u128::from(ip) & mask) == (u128::from(net) & mask)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{WebAuthConfig, WebConfig, WebProbesConfig};

    #[test]
    fn cidr_match() {
        assert!(ip_in_cidr("10.0.0.5", "10.0.0.0/8"));
        assert!(!ip_in_cidr("203.0.113.1", "10.0.0.0/8"));
    }

    #[test]
    fn probe_paths_match_config() {
        let web = WebConfig {
            probes: WebProbesConfig {
                enabled: true,
                paths: vec!["/.env".into(), "/wp-admin".into()],
            },
            auth: WebAuthConfig::default(),
        };
        assert!(matches_any_path("/.env", &web.probes.paths));
        assert!(matches_any_path("/wp-admin/setup.php", &web.probes.paths));
    }
}
