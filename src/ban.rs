use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::config::{BanConfig, SshConfig, WhitelistConfig};
use crate::crawler::PathCatalog;
use crate::log_parser::AccessEntry;
use crate::nft::NftBanClient;
use crate::ssh_parser::SshEvent;

#[derive(Clone)]
pub struct BanService {
    config: BanConfig,
    ssh_config: SshConfig,
    whitelist: WhitelistConfig,
    nft: NftBanClient,
    offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
    ssh_offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl BanService {
    pub fn new(
        config: BanConfig,
        ssh_config: SshConfig,
        whitelist: WhitelistConfig,
        nft: NftBanClient,
    ) -> Self {
        Self {
            config,
            ssh_config,
            whitelist,
            nft,
            offenders: Arc::new(Mutex::new(HashMap::new())),
            ssh_offenders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn handle_entry(
        &self,
        site: &str,
        entry: AccessEntry,
        catalog: &PathCatalog,
    ) -> Result<()> {
        if entry.status != 404 {
            return Ok(());
        }

        if catalog.contains(&entry.path).await {
            return Ok(());
        }

        if self.is_whitelisted(&entry.ip) {
            return Ok(());
        }

        let count = record_hit(&self.offenders, &entry.ip, self.config.window_secs).await;
        if count < self.config.threshold {
            return Ok(());
        }

        self.offenders.lock().await.remove(&entry.ip);

        if self.config.dry_run {
            warn!(
                site,
                ip = %entry.ip,
                path = %entry.path,
                count,
                "dry-run: would ban IP for probing unknown pages"
            );
            return Ok(());
        }

        self.nft.ban_ip(&entry.ip).await?;
        info!(
            site,
            ip = %entry.ip,
            path = %entry.path,
            hits = count,
            "IP banned for probing unknown pages"
        );
        Ok(())
    }

    pub async fn handle_ssh_event(&self, event: SshEvent) -> Result<()> {
        if !self.ssh_config.enabled {
            return Ok(());
        }

        match event {
            SshEvent::InvalidUser { ip } => self.ban_ssh_invalid_user(&ip).await,
            SshEvent::FailedPassword { ip } => self.ban_ssh_failed_password(&ip).await,
        }
    }

    async fn ban_ssh_invalid_user(&self, ip: &str) -> Result<()> {
        if self.is_whitelisted(ip) {
            return Ok(());
        }

        self.ssh_offenders.lock().await.remove(ip);

        if self.config.dry_run {
            warn!(
                ip = %ip,
                "dry-run: would ban IP for invalid SSH user"
            );
            return Ok(());
        }

        self.nft.ban_ip(ip).await?;
        info!(ip = %ip, "IP banned for invalid SSH user");
        Ok(())
    }

    async fn ban_ssh_failed_password(&self, ip: &str) -> Result<()> {
        if self.is_whitelisted(ip) {
            return Ok(());
        }

        let count = record_hit(&self.ssh_offenders, ip, self.ssh_config.window_secs).await;
        if count < self.ssh_config.threshold {
            return Ok(());
        }

        self.ssh_offenders.lock().await.remove(ip);

        if self.config.dry_run {
            warn!(
                ip = %ip,
                count,
                "dry-run: would ban IP for SSH brute force"
            );
            return Ok(());
        }

        self.nft.ban_ip(ip).await?;
        info!(ip = %ip, hits = count, "IP banned for SSH brute force");
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

    #[test]
    fn cidr_match() {
        assert!(ip_in_cidr("10.0.0.5", "10.0.0.0/8"));
        assert!(!ip_in_cidr("203.0.113.1", "10.0.0.0/8"));
    }
}
