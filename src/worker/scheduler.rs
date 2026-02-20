use std::str::FromStr;
use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::models::schedule::Schedule;

pub async fn run_scheduler(
    writer: SqlitePool,
    reader: SqlitePool,
    interval_secs: u64,
    cancel: CancellationToken,
) {
    let interval = Duration::from_secs(interval_secs);
    info!(interval_secs, "Scheduler started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Scheduler shutting down");
                return;
            }
            _ = tokio::time::sleep(interval) => {
                if let Err(e) = tick(&writer, &reader).await {
                    warn!(error = %e, "Scheduler tick failed");
                }
            }
        }
    }
}

async fn tick(writer: &SqlitePool, reader: &SqlitePool) -> anyhow::Result<()> {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();

    let due: Vec<Schedule> = sqlx::query_as(
        "SELECT * FROM schedules WHERE enabled = 1 AND next_run_at <= ?1",
    )
    .bind(&now)
    .fetch_all(reader)
    .await?;

    for schedule in &due {
        // Compute next_run_at
        let next_run = match cron::Schedule::from_str(&schedule.cron_expression) {
            Ok(cron_sched) => {
                match cron_sched.upcoming(Utc).next() {
                    Some(dt) => dt.format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
                    None => {
                        warn!(schedule_id = %schedule.id, "No upcoming runs for cron expression");
                        continue;
                    }
                }
            }
            Err(e) => {
                warn!(schedule_id = %schedule.id, error = %e, "Invalid cron expression");
                continue;
            }
        };

        // CAS update: only succeeds if next_run_at hasn't changed (race condition mitigation)
        let updated = sqlx::query(
            r#"UPDATE schedules
               SET last_run_at = ?1,
                   next_run_at = ?2,
                   updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
               WHERE id = ?3 AND next_run_at = ?4"#,
        )
        .bind(&now)
        .bind(&next_run)
        .bind(&schedule.id)
        .bind(&schedule.next_run_at)
        .execute(writer)
        .await?;

        if updated.rows_affected() == 0 {
            // Another instance already processed this schedule
            continue;
        }

        // Insert job
        let job_id = uuid::Uuid::now_v7().to_string();
        sqlx::query(
            r#"INSERT INTO jobs (id, queue_name, payload, callback_url, max_retries, timeout_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
        )
        .bind(&job_id)
        .bind(&schedule.queue_name)
        .bind(&schedule.payload)
        .bind(&schedule.callback_url)
        .bind(schedule.max_retries)
        .bind(schedule.timeout_ms)
        .execute(writer)
        .await?;

        // Ensure queue exists
        sqlx::query("INSERT OR IGNORE INTO queues (name) VALUES (?1)")
            .bind(&schedule.queue_name)
            .execute(writer)
            .await?;

        info!(
            schedule_id = %schedule.id,
            job_id = %job_id,
            queue = %schedule.queue_name,
            next_run = %next_run,
            "Scheduled job created"
        );
    }

    Ok(())
}
