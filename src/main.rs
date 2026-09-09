use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "caddyban", about = "Ban IPs probing web pages and SSH brute-force attempts")]
struct Cli {
    #[arg(short, long, default_value = "/etc/caddyban/config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the ban daemon (default).
    Run,
    /// Crawl configured sites once and print discovered paths.
    Crawl,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("caddyban=info".parse()?))
        .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => caddyban::run(&cli.config).await?,
        Commands::Crawl => {
            let sites = caddyban::crawl_only(&cli.config).await?;
            let site_count = sites.len();
            let mut total = 0usize;
            for (site, paths) in &sites {
                for path in paths {
                    println!("[{site}] {path}");
                }
                total += paths.len();
            }
            tracing::info!(sites = site_count, pages = total, "crawl finished");
        }
    }

    Ok(())
}
