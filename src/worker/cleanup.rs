use std::time::Duration;

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

pub async fn run_cleanup(
    writer: SqlitePool,
    retention_days: u32,
    interval_secs: u64,
    cancel: CancellationToken,
) {
    let interval = Duration::from_secs(interval_secs);
    info!(retention_days, interval_secs, "Cleanup task started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Cleanup task shutting down");
                return;
            }
            _ = tokio::time::sleep(interval) => {
                if let Err(e) = delete_old_jobs(&writer, retention_days).await {
                    warn!(error = %e, "Cleanup cycle failed");
                }
            }
        }
    }
}

async fn delete_old_jobs(writer: &SqlitePool, retention_days: u32) -> anyhow::Result<()> {
    let result = sqlx::query(
        r#"DELETE FROM jobs
           WHERE status IN ('completed', 'dead')
             AND completed_at < strftime('%Y-%m-%dT%H:%M:%f', 'now', ?1)"#,
    )
    .bind(format!("-{retention_days} days"))
    .execute(writer)
    .await?;

    let count = result.rows_affected();
    if count > 0 {
        info!(count, retention_days, "Cleaned up old jobs");
    }

    Ok(())
}
