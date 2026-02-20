use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::Config;
use crate::db::DbPools;
use crate::models::job::Job;
use crate::worker::executor::{execute_webhook, WebhookResult};
use crate::worker::rate_limiter::RateLimiter;
use crate::worker::retry::compute_delay_ms;

pub struct WorkerEngine {
    db: DbPools,
    http_client: reqwest::Client,
    worker_id: String,
    config: Config,
    semaphore: Arc<Semaphore>,
    cancel: CancellationToken,
    rate_limiter: Mutex<RateLimiter>,
}

impl WorkerEngine {
    pub fn new(
        db: DbPools,
        http_client: reqwest::Client,
        config: Config,
        cancel: CancellationToken,
    ) -> Self {
        let concurrency = config.worker.concurrency;
        let worker_id = uuid::Uuid::now_v7().to_string();
        info!(worker_id = %worker_id, concurrency, "Worker engine created");

        Self {
            db,
            http_client,
            worker_id,
            config,
            semaphore: Arc::new(Semaphore::new(concurrency)),
            cancel,
            rate_limiter: Mutex::new(RateLimiter::new()),
        }
    }

    pub async fn run(&self) {
        let poll_interval =
            Duration::from_millis(self.config.worker.poll_interval_ms);

        info!(
            worker_id = %self.worker_id,
            poll_ms = self.config.worker.poll_interval_ms,
            "Worker engine started"
        );

        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    info!("Worker engine shutting down, waiting for in-flight jobs...");
                    let _ = self.semaphore.acquire_many(self.config.worker.concurrency as u32).await;
                    info!("All in-flight jobs completed");
                    return;
                }
                _ = tokio::time::sleep(poll_interval) => {
                    self.poll_and_execute().await;
                }
            }
        }
    }

    async fn poll_and_execute(&self) {
        let permit = match self.semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => return,
        };

        let job = match self.claim_job().await {
            Ok(Some(job)) => job,
            Ok(None) => {
                drop(permit);
                return;
            }
            Err(e) => {
                warn!(error = %e, "Failed to claim job");
                drop(permit);
                return;
            }
        };

        let db = self.db.clone();
        let client = self.http_client.clone();
        let config = self.config.clone();
        let job_id = job.id.clone();

        tokio::spawn(async move {
            let _permit = permit;

            info!(job_id = %job_id, queue = %job.queue_name, attempt = job.attempt, "Executing job");
            let result = execute_webhook(&client, &job).await;
            if let Err(e) = handle_result(&db, &config, &job, result).await {
                warn!(job_id = %job_id, error = %e, "Failed to handle job result");
            }
        });
    }

    async fn claim_job(&self) -> anyhow::Result<Option<Job>> {
        // Get list of queues to poll with their rate limits
        let queues: Vec<QueueInfo> = if self.config.worker.queues.is_empty() {
            sqlx::query_as(
                "SELECT name, rate_limit_rps FROM queues WHERE paused = 0",
            )
            .fetch_all(&self.db.reader)
            .await?
        } else {
            // When specific queues are configured, still fetch rate limits
            let configured = &self.config.worker.queues;
            let placeholders: String = (1..=configured.len())
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(",");
            let query = format!(
                "SELECT name, rate_limit_rps FROM queues WHERE paused = 0 AND name IN ({placeholders})"
            );
            let mut q = sqlx::query_as::<_, QueueInfo>(&query);
            for name in configured {
                q = q.bind(name);
            }
            q.fetch_all(&self.db.reader).await?
        };

        if queues.is_empty() {
            return Ok(None);
        }

        // Filter through rate limiter
        let allowed: Vec<String> = {
            let mut rl = self.rate_limiter.lock().unwrap();
            queues
                .into_iter()
                .filter(|qi| rl.check_queue(&qi.name, qi.rate_limit_rps))
                .map(|qi| qi.name)
                .collect()
        };

        if allowed.is_empty() {
            return Ok(None);
        }

        // Randomize queue order to avoid starvation
        use rand::seq::SliceRandom;
        let mut allowed = allowed;
        allowed.shuffle(&mut rand::thread_rng());

        for queue_name in &allowed {
            let job: Option<Job> = sqlx::query_as(
                r#"UPDATE jobs
                   SET status = 'running',
                       visible_at = strftime('%Y-%m-%dT%H:%M:%f', 'now',
                           '+' || (timeout_ms / 1000) || ' seconds'),
                       started_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'),
                       worker_id = ?1,
                       attempt = attempt + 1,
                       updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
                   WHERE id = (
                       SELECT id FROM jobs
                       WHERE status = 'pending'
                         AND visible_at <= strftime('%Y-%m-%dT%H:%M:%f', 'now')
                         AND queue_name = ?2
                       ORDER BY priority DESC, created_at ASC
                       LIMIT 1
                   ) RETURNING *"#,
            )
            .bind(&self.worker_id)
            .bind(queue_name)
            .fetch_optional(&self.db.writer)
            .await?;

            if job.is_some() {
                return Ok(job);
            }
        }

        Ok(None)
    }
}

#[derive(sqlx::FromRow)]
struct QueueInfo {
    name: String,
    rate_limit_rps: Option<f64>,
}

async fn handle_result(
    db: &DbPools,
    config: &Config,
    job: &Job,
    result: WebhookResult,
) -> anyhow::Result<()> {
    match result {
        WebhookResult::Success(body) => {
            sqlx::query(
                r#"UPDATE jobs
                   SET status = 'completed',
                       completed_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'),
                       result = ?2,
                       updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
                   WHERE id = ?1"#,
            )
            .bind(&job.id)
            .bind(body)
            .execute(&db.writer)
            .await?;

            info!(job_id = %job.id, "Job completed successfully");
        }
        WebhookResult::PermanentFailure(error) => {
            dead_letter(db, job, &error).await?;
        }
        WebhookResult::TransientFailure(error)
        | WebhookResult::ConnectionError(error) => {
            retry_or_dead_letter(db, config, job, &error).await?;
        }
        WebhookResult::Timeout => {
            retry_or_dead_letter(db, config, job, "Webhook timeout").await?;
        }
    }

    Ok(())
}

async fn retry_or_dead_letter(
    db: &DbPools,
    config: &Config,
    job: &Job,
    error: &str,
) -> anyhow::Result<()> {
    let max_retries = job.max_retries;

    if job.attempt >= max_retries {
        dead_letter(db, job, error).await?;
        return Ok(());
    }

    let backoff = job
        .retry_backoff
        .as_deref()
        .unwrap_or(&config.defaults.retry_backoff);
    let base_delay = job.base_delay_ms.unwrap_or(config.defaults.base_delay_ms);
    let max_delay = job.max_delay_ms.unwrap_or(config.defaults.max_delay_ms);

    let delay_ms = compute_delay_ms(job.attempt, backoff, base_delay, max_delay);
    let delay_secs = delay_ms as f64 / 1000.0;

    let visible_at_expr = format!(
        "strftime('%Y-%m-%dT%H:%M:%f', 'now', '+{delay_secs} seconds')"
    );

    let query = format!(
        r#"UPDATE jobs
           SET status = 'pending',
               visible_at = {visible_at_expr},
               last_error = ?2,
               worker_id = NULL,
               started_at = NULL,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE id = ?1"#
    );

    sqlx::query(&query)
        .bind(&job.id)
        .bind(error)
        .execute(&db.writer)
        .await?;

    warn!(
        job_id = %job.id,
        attempt = job.attempt,
        max_retries,
        delay_ms,
        error,
        "Job will be retried"
    );

    Ok(())
}

async fn dead_letter(
    db: &DbPools,
    job: &Job,
    error: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"UPDATE jobs
           SET status = 'dead',
               completed_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'),
               last_error = ?2,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE id = ?1"#,
    )
    .bind(&job.id)
    .bind(error)
    .execute(&db.writer)
    .await?;

    warn!(
        job_id = %job.id,
        attempt = job.attempt,
        error,
        "Job dead-lettered"
    );

    Ok(())
}
