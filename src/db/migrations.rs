use sqlx::SqlitePool;
use tracing::info;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS queues (
    name            TEXT PRIMARY KEY NOT NULL,
    max_concurrency INTEGER NOT NULL DEFAULT 5,
    rate_limit_rps  REAL,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    retry_backoff   TEXT NOT NULL DEFAULT 'exponential',
    base_delay_ms   INTEGER NOT NULL DEFAULT 1000,
    max_delay_ms    INTEGER NOT NULL DEFAULT 300000,
    paused          INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE TABLE IF NOT EXISTS jobs (
    id              TEXT PRIMARY KEY NOT NULL,
    queue_name      TEXT NOT NULL DEFAULT 'default',
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','running','completed','dead')),
    priority        INTEGER NOT NULL DEFAULT 0,
    payload         TEXT NOT NULL DEFAULT '{}',
    callback_url    TEXT NOT NULL,
    visible_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    attempt         INTEGER NOT NULL DEFAULT 0,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    started_at      TEXT,
    completed_at    TEXT,
    worker_id       TEXT,
    retry_backoff   TEXT,
    base_delay_ms   INTEGER,
    max_delay_ms    INTEGER,
    last_error      TEXT,
    result          TEXT,
    idempotency_key TEXT,
    timeout_ms      INTEGER NOT NULL DEFAULT 30000,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_jobs_poll
    ON jobs (queue_name, status, visible_at, priority DESC, created_at ASC)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_jobs_reaper
    ON jobs (status, visible_at) WHERE status = 'running';

CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_idempotency
    ON jobs (idempotency_key) WHERE idempotency_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS schedules (
    id              TEXT PRIMARY KEY NOT NULL,
    queue_name      TEXT NOT NULL DEFAULT 'default',
    cron_expression TEXT NOT NULL,
    callback_url    TEXT NOT NULL,
    payload         TEXT NOT NULL DEFAULT '{}',
    max_retries     INTEGER NOT NULL DEFAULT 3,
    timeout_ms      INTEGER NOT NULL DEFAULT 30000,
    enabled         INTEGER NOT NULL DEFAULT 1,
    last_run_at     TEXT,
    next_run_at     TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_schedules_next_run
    ON schedules (enabled, next_run_at) WHERE enabled = 1;
"#;

pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql(SCHEMA).execute(pool).await?;
    info!("Database migrations applied");
    Ok(())
}
