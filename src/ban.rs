use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::config::{BanConfig, WhitelistConfig};
use crate::crawler::PathCatalog;
use crate::log_parser::AccessEntry;
use crate::nft::NftBanClient;

#[derive(Clone)]
pub struct BanService {
    config: BanConfig,
    whitelist: WhitelistConfig,
    nft: NftBanClient,
    offenders: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl BanService {
    pub fn new(config: BanConfig, whitelist: WhitelistConfig, nft: NftBanClient) -> Self {
        Self {
            config,
            whitelist,
            nft,
            offenders: Arc::new(Mutex::new(HashMap::new())),
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

        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_secs);
        let mut offenders = self.offenders.lock().await;
        let hits = offenders.entry(entry.ip.clone()).or_default();
        hits.push_back(now);
        while let Some(front) = hits.front() {
            if now.duration_since(*front) > window {
                hits.pop_front();
            } else {
                break;
            }
        }

        let count = hits.len() as u32;
        if count < self.config.threshold {
            return Ok(());
        }

        hits.clear();
        drop(offenders);

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
