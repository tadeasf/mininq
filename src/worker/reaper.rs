use std::time::Duration;

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub async fn run_reaper(
    writer: SqlitePool,
    interval_secs: u64,
    cancel: CancellationToken,
) {
    let interval = Duration::from_secs(interval_secs);
    info!(interval_secs, "Reaper started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Reaper shutting down");
                return;
            }
            _ = tokio::time::sleep(interval) => {
                if let Err(e) = reap_stale_jobs(&writer).await {
                    warn!(error = %e, "Reaper cycle failed");
                }
            }
        }
    }
}

async fn reap_stale_jobs(writer: &SqlitePool) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"UPDATE jobs
           SET status = 'pending',
               worker_id = NULL,
               started_at = NULL,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE status = 'running'
             AND visible_at < strftime('%Y-%m-%dT%H:%M:%f', 'now')"#,
    )
    .execute(writer)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        warn!(count, "Reaped stale jobs");
    }

    Ok(())
}
