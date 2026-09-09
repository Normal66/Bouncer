use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bouncer",
    about = "Server bouncer — ban web scanners and SSH brute-forcers"
)]
struct Cli {
    #[arg(short, long, default_value = "/etc/bouncer/config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the bouncer daemon (default).
    Run,
    /// Crawl configured sites once and print discovered paths.
    Crawl,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("bouncer=info".parse()?))
        .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => bouncer::run(&cli.config).await?,
        Commands::Crawl => {
            let sites = bouncer::crawl_only(&cli.config).await?;
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
