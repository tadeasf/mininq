use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::api::error::ApiError;
use crate::AppState;

#[derive(Serialize)]
pub struct MetricsResponse {
    pub uptime_secs: u64,
    pub version: &'static str,
    pub jobs: JobMetrics,
    pub queues: Vec<QueueMetric>,
    pub schedules: ScheduleMetrics,
}

#[derive(Serialize)]
pub struct JobMetrics {
    pub total: i64,
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub dead: i64,
}

#[derive(Serialize)]
pub struct QueueMetric {
    pub name: String,
    pub paused: bool,
    pub depth: i64,
    pub in_flight: i64,
}

#[derive(Serialize)]
pub struct ScheduleMetrics {
    pub total: i64,
    pub enabled: i64,
}

pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricsResponse>, ApiError> {
    let uptime_secs = state.start_time.elapsed().as_secs();

    // Job counts
    let job_counts: JobCountsRow = sqlx::query_as(
        r#"SELECT
            COUNT(*) as total,
            COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
            COALESCE(SUM(CASE WHEN status = 'running' THEN 1 ELSE 0 END), 0) as running,
            COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed,
            COALESCE(SUM(CASE WHEN status = 'dead' THEN 1 ELSE 0 END), 0) as dead
           FROM jobs"#,
    )
    .fetch_one(&state.db.reader)
    .await?;

    // Queue stats
    let queue_rows: Vec<QueueMetricRow> = sqlx::query_as(
        r#"SELECT
            q.name,
            q.paused,
            COALESCE(SUM(CASE WHEN j.status = 'pending' THEN 1 ELSE 0 END), 0) as depth,
            COALESCE(SUM(CASE WHEN j.status = 'running' THEN 1 ELSE 0 END), 0) as in_flight
           FROM queues q
           LEFT JOIN jobs j ON j.queue_name = q.name
           GROUP BY q.name
           ORDER BY q.name"#,
    )
    .fetch_all(&state.db.reader)
    .await?;

    // Schedule counts
    let schedule_counts: ScheduleCountsRow = sqlx::query_as(
        r#"SELECT
            COUNT(*) as total,
            COALESCE(SUM(CASE WHEN enabled = 1 THEN 1 ELSE 0 END), 0) as enabled
           FROM schedules"#,
    )
    .fetch_one(&state.db.reader)
    .await?;

    Ok(Json(MetricsResponse {
        uptime_secs,
        version: env!("CARGO_PKG_VERSION"),
        jobs: JobMetrics {
            total: job_counts.total,
            pending: job_counts.pending,
            running: job_counts.running,
            completed: job_counts.completed,
            dead: job_counts.dead,
        },
        queues: queue_rows
            .into_iter()
            .map(|r| QueueMetric {
                name: r.name,
                paused: r.paused != 0,
                depth: r.depth,
                in_flight: r.in_flight,
            })
            .collect(),
        schedules: ScheduleMetrics {
            total: schedule_counts.total,
            enabled: schedule_counts.enabled,
        },
    }))
}

#[derive(sqlx::FromRow)]
struct JobCountsRow {
    total: i64,
    pending: i64,
    running: i64,
    completed: i64,
    dead: i64,
}

#[derive(sqlx::FromRow)]
struct QueueMetricRow {
    name: String,
    paused: i32,
    depth: i64,
    in_flight: i64,
}

#[derive(sqlx::FromRow)]
struct ScheduleCountsRow {
    total: i64,
    enabled: i64,
}
