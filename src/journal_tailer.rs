use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::warn;

pub async fn tail_journal_unit(
    unit: &str,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<mpsc::Receiver<String>> {
    let (tx, rx) = mpsc::channel(1024);

    let unit = unit.to_owned();
    tokio::spawn(async move {
        if let Err(err) = journal_loop(&unit, &mut shutdown, tx).await {
            warn!(error = %err, unit = %unit, "journal tailer stopped");
        }
    });

    Ok(rx)
}

async fn journal_loop(
    unit: &str,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    tx: mpsc::Sender<String>,
) -> Result<()> {
    loop {
        if *shutdown.borrow() {
            break;
        }

        let mut child = Command::new("journalctl")
            .args(["-f", "-n", "0", "-u", unit, "--no-pager", "-o", "cat"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("failed to spawn journalctl")?;

        let stdout = child
            .stdout
            .take()
            .context("journalctl stdout not captured")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        loop {
            line.clear();
            tokio::select! {
                read = reader.read_line(&mut line) => {
                    match read {
                        Ok(0) => break,
                        Ok(_) => {
                            let trimmed = line.trim_end_matches(['\r', '\n']);
                            if !trimmed.is_empty() && tx.send(trimmed.to_owned()).await.is_err() {
                                let _ = child.kill().await;
                                return Ok(());
                            }
                        }
                        Err(err) => return Err(err.into()),
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_ok() && *shutdown.borrow() {
                        let _ = child.kill().await;
                        return Ok(());
                    }
                }
            }
        }

        let status = child.wait().await?;
        if *shutdown.borrow() {
            break;
        }

        warn!(
            unit,
            status = ?status,
            "journalctl exited, restarting in 5 seconds"
        );
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}
