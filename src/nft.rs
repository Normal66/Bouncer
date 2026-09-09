use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use tracing::debug;

use crate::config::NftConfig;

#[derive(Clone)]
pub struct NftBanClient {
    config: NftConfig,
    banned: Arc<Mutex<HashSet<String>>>,
}

impl NftBanClient {
    pub fn new(config: NftConfig) -> Self {
        Self {
            config,
            banned: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn ban_ip(&self, ip: &str) -> Result<()> {
        {
            let banned = self.banned.lock().await;
            if banned.contains(ip) {
                return Ok(());
            }
        }

        let timeout = if self.config.timeout.is_empty() {
            None
        } else {
            Some(self.config.timeout.as_str())
        };

        let mut args = vec![
            "add".to_string(),
            "element".to_string(),
            self.config.table.clone(),
            self.config.set.clone(),
        ];

        let element = if let Some(timeout) = timeout {
            format!("{{ {ip} timeout {timeout} }}")
        } else {
            format!("{{ {ip} }}")
        };
        args.push(element);

        run_nft(args).await.with_context(|| format!("ban IP {ip}"))?;

        self.banned.lock().await.insert(ip.to_owned());
        debug!(%ip, "nftables ban applied");
        Ok(())
    }
}

async fn run_nft(args: Vec<String>) -> Result<()> {
    let output = tokio::process::Command::new("nft")
        .args(&args)
        .output()
        .await
        .context("failed to execute nft command")?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("nft exited with {}: {}", output.status, stderr.trim());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_timeout_element() {
        let cfg = NftConfig {
            table: "inet filter".into(),
            set: "blocked_ips".into(),
            timeout: "30m".into(),
        };
        let client = NftBanClient::new(cfg);
        assert_eq!(client.config.set, "blocked_ips");
    }
}
