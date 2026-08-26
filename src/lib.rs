pub mod ban;
pub mod config;
pub mod crawler;
pub mod log_parser;
pub mod log_tailer;
pub mod nft;

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::watch;
use tracing::{info, warn};

use crate::ban::BanService;
use crate::config::{Config, SiteEntry};
use crate::crawler::{PathCatalog, crawl_site};
use crate::log_parser::parse_line;
use crate::log_tailer::tail_file;
use crate::nft::NftBanClient;

pub async fn run(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;
    info!(sites = config.sites.len(), "caddyban started");

    let ban_service = BanService::new(
        config.ban.clone(),
        config.whitelist.clone(),
        NftBanClient::new(config.nft.clone()),
    );

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut stop = shutdown_tx.subscribe();

    for site in &config.sites {
        spawn_site(&config, site, ban_service.clone(), shutdown_tx.clone(), shutdown_rx.clone())
            .await?;
    }

    tokio::select! {
        _ = wait_for_shutdown(&mut stop) => {
            info!("shutdown signal received");
        }
    }

    let _ = shutdown_tx.send(true);
    Ok(())
}

async fn spawn_site(
    config: &Config,
    site: &SiteEntry,
    ban_service: BanService,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    let label = site.label().to_owned();
    let crawl_settings = site.crawl_config(&config.crawl);

    let initial_paths = crawl_site(&crawl_settings)
        .await
        .with_context(|| format!("initial crawl failed for {label}"))?;
    let catalog = PathCatalog::new(initial_paths);
    info!(site = %label, pages = catalog.len().await, "path catalog ready");

    let crawl_catalog = catalog.clone();
    let crawl_settings = Arc::new(crawl_settings);
    let crawl_interval = config.crawl_interval();
    let crawl_label = label.clone();
    let mut crawl_shutdown = shutdown_tx.subscribe();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = tokio::time::sleep(crawl_interval) => {
                    match crawl_site(crawl_settings.as_ref()).await {
                        Ok(paths) => crawl_catalog.replace(paths).await,
                        Err(err) => warn!(site = %crawl_label, error = %err, "scheduled crawl failed"),
                    }
                }
                changed = crawl_shutdown.changed() => {
                    if changed.is_ok() && *crawl_shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    });

    let mut log_lines = tail_file(
        &site.log_path,
        config.log_poll_interval(),
        shutdown_rx,
    )
    .await
    .with_context(|| format!("failed to tail {}", site.log_path.display()))?;

    let log_format = config.logs.format;
    let site_label = label.clone();
    tokio::spawn(async move {
        while let Some(line) = log_lines.recv().await {
            match parse_line(log_format, &line) {
                Ok(Some(entry)) => {
                    if let Err(err) = ban_service
                        .handle_entry(&site_label, entry, &catalog)
                        .await
                    {
                        warn!(site = %site_label, error = %err, "failed to process access log entry");
                    }
                }
                Ok(None) => {}
                Err(err) => warn!(site = %site_label, error = %err, "failed to parse log line"),
            }
        }
    });

    Ok(())
}

async fn wait_for_shutdown(stop: &mut watch::Receiver<bool>) -> Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate())?;
        let mut interrupt = signal(SignalKind::interrupt())?;
        tokio::select! {
            _ = terminate.recv() => {}
            _ = interrupt.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .context("failed to listen for shutdown signal")?;
    }
    let _ = stop.changed().await;
    Ok(())
}

pub async fn crawl_only(config_path: &Path) -> Result<Vec<(String, HashSet<String>)>> {
    let config = Config::load(config_path)?;
    let mut results = Vec::with_capacity(config.sites.len());

    for site in &config.sites {
        let paths = crawl_site(&site.crawl_config(&config.crawl))
            .await
            .with_context(|| format!("crawl failed for {}", site.label()))?;
        results.push((site.label().to_owned(), paths));
    }

    Ok(results)
}
