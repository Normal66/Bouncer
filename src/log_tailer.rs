use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::sync::mpsc;
use tracing::{debug, warn};

pub async fn tail_file(
    path: &Path,
    poll_interval: Duration,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<mpsc::Receiver<String>> {
    let (tx, rx) = mpsc::channel(1024);

    let path = path.to_owned();
    tokio::spawn(async move {
        if let Err(err) = tail_loop(&path, poll_interval, &mut shutdown, tx).await {
            warn!(error = %err, path = %path.display(), "log tailer stopped");
        }
    });

    Ok(rx)
}

async fn tail_loop(
    path: &Path,
    poll_interval: Duration,
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    tx: mpsc::Sender<String>,
) -> Result<()> {
    let mut offset = file_size(path).await.unwrap_or(0);
    debug!(offset, path = %path.display(), "starting log tail at end of file");

    loop {
        if *shutdown.borrow() {
            break;
        }

        match read_from_offset(path, &mut offset).await {
            Ok(lines) => {
                for line in lines {
                    if tx.send(line).await.is_err() {
                        return Ok(());
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, path = %path.display(), "failed to read log file");
            }
        }

        tokio::select! {
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(poll_interval) => {}
        }
    }

    Ok(())
}

async fn file_size(path: &Path) -> Result<u64> {
    let metadata = tokio::fs::metadata(path)
        .await
        .with_context(|| format!("failed to stat {}", path.display()))?;
    Ok(metadata.len())
}

async fn read_from_offset(path: &Path, offset: &mut u64) -> Result<Vec<String>> {
    let mut file = File::open(path)
        .await
        .with_context(|| format!("failed to open {}", path.display()))?;

    let size = file.metadata().await?.len();
    if size < *offset {
        debug!(old_offset = *offset, new_size = size, "log file truncated, rewinding");
        *offset = 0;
    }

    file.seek(SeekFrom::Start(*offset)).await?;
    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut buffer = String::new();

    loop {
        buffer.clear();
        let read = reader.read_line(&mut buffer).await?;
        if read == 0 {
            break;
        }
        *offset += read as u64;
        if buffer.ends_with('\n') {
            lines.push(buffer.trim_end_matches(['\r', '\n']).to_owned());
        }
    }

    Ok(lines)
}
