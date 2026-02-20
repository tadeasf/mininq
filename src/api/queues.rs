use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::api::error::ApiError;
use crate::models::queue::{CreateQueueRequest, Queue, QueueStats, UpdateQueueRequest};
use crate::AppState;

pub async fn list_queues(
    State(state): State<AppState>,
) -> Result<Json<Vec<QueueStats>>, ApiError> {
    let rows: Vec<QueueStatsRow> = sqlx::query_as(
        r#"SELECT
            q.name,
            q.max_concurrency,
            q.paused,
            COALESCE(SUM(CASE WHEN j.status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
            COALESCE(SUM(CASE WHEN j.status = 'running' THEN 1 ELSE 0 END), 0) as running,
            COALESCE(SUM(CASE WHEN j.status = 'completed' THEN 1 ELSE 0 END), 0) as completed,
            COALESCE(SUM(CASE WHEN j.status = 'dead' THEN 1 ELSE 0 END), 0) as dead
           FROM queues q
           LEFT JOIN jobs j ON j.queue_name = q.name
           GROUP BY q.name
           ORDER BY q.name"#,
    )
    .fetch_all(&state.db.reader)
    .await?;

    let stats = rows
        .into_iter()
        .map(|r| QueueStats {
            name: r.name,
            max_concurrency: r.max_concurrency,
            paused: r.paused != 0,
            pending: r.pending,
            running: r.running,
            completed: r.completed,
            dead: r.dead,
        })
        .collect();

    Ok(Json(stats))
}

pub async fn get_queue(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<QueueStats>, ApiError> {
    let row: QueueStatsRow = sqlx::query_as(
        r#"SELECT
            q.name,
            q.max_concurrency,
            q.paused,
            COALESCE(SUM(CASE WHEN j.status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
            COALESCE(SUM(CASE WHEN j.status = 'running' THEN 1 ELSE 0 END), 0) as running,
            COALESCE(SUM(CASE WHEN j.status = 'completed' THEN 1 ELSE 0 END), 0) as completed,
            COALESCE(SUM(CASE WHEN j.status = 'dead' THEN 1 ELSE 0 END), 0) as dead
           FROM queues q
           LEFT JOIN jobs j ON j.queue_name = q.name
           WHERE q.name = ?1
           GROUP BY q.name"#,
    )
    .bind(&name)
    .fetch_optional(&state.db.reader)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Queue '{name}' not found")))?;

    Ok(Json(QueueStats {
        name: row.name,
        max_concurrency: row.max_concurrency,
        paused: row.paused != 0,
        pending: row.pending,
        running: row.running,
        completed: row.completed,
        dead: row.dead,
    }))
}

pub async fn create_queue(
    State(state): State<AppState>,
    Json(req): Json<CreateQueueRequest>,
) -> Result<(StatusCode, Json<Queue>), ApiError> {
    if req.name.is_empty() {
        return Err(ApiError::BadRequest("Queue name is required".to_string()));
    }

    let queue: Queue = sqlx::query_as(
        r#"INSERT INTO queues (name, max_concurrency, rate_limit_rps, max_retries,
                               retry_backoff, base_delay_ms, max_delay_ms)
           VALUES (?1,
                   COALESCE(?2, 5),
                   ?3,
                   COALESCE(?4, 3),
                   COALESCE(?5, 'exponential'),
                   COALESCE(?6, 1000),
                   COALESCE(?7, 300000))
           RETURNING *"#,
    )
    .bind(&req.name)
    .bind(req.max_concurrency)
    .bind(req.rate_limit_rps)
    .bind(req.max_retries)
    .bind(req.retry_backoff.as_deref())
    .bind(req.base_delay_ms)
    .bind(req.max_delay_ms)
    .fetch_one(&state.db.writer)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            ApiError::Conflict(format!("Queue '{}' already exists", req.name))
        }
        other => ApiError::Database(other),
    })?;

    tracing::info!(queue = %queue.name, "Queue created");
    Ok((StatusCode::CREATED, Json(queue)))
}

pub async fn update_queue(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateQueueRequest>,
) -> Result<Json<Queue>, ApiError> {
    let mut sets = Vec::new();
    let mut bind_idx = 1u32;

    // We'll build the query dynamically
    struct BindValue {
        int: Option<i32>,
        float: Option<f64>,
        text: Option<String>,
    }
    let mut binds: Vec<BindValue> = Vec::new();

    if let Some(v) = req.max_concurrency {
        bind_idx += 1;
        sets.push(format!("max_concurrency = ?{bind_idx}"));
        binds.push(BindValue { int: Some(v), float: None, text: None });
    }
    if let Some(v) = req.rate_limit_rps {
        bind_idx += 1;
        sets.push(format!("rate_limit_rps = ?{bind_idx}"));
        binds.push(BindValue { int: None, float: Some(v), text: None });
    }
    if let Some(v) = req.max_retries {
        bind_idx += 1;
        sets.push(format!("max_retries = ?{bind_idx}"));
        binds.push(BindValue { int: Some(v), float: None, text: None });
    }
    if let Some(ref v) = req.retry_backoff {
        bind_idx += 1;
        sets.push(format!("retry_backoff = ?{bind_idx}"));
        binds.push(BindValue { int: None, float: None, text: Some(v.clone()) });
    }
    if let Some(v) = req.base_delay_ms {
        bind_idx += 1;
        sets.push(format!("base_delay_ms = ?{bind_idx}"));
        binds.push(BindValue { int: Some(v), float: None, text: None });
    }
    if let Some(v) = req.max_delay_ms {
        bind_idx += 1;
        sets.push(format!("max_delay_ms = ?{bind_idx}"));
        binds.push(BindValue { int: Some(v), float: None, text: None });
    }

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')".to_string());

    let query = format!(
        "UPDATE queues SET {} WHERE name = ?1 RETURNING *",
        sets.join(", ")
    );

    let mut q = sqlx::query_as::<_, Queue>(&query).bind(&name);
    for b in &binds {
        if let Some(v) = b.int {
            q = q.bind(v);
        } else if let Some(v) = b.float {
            q = q.bind(v);
        } else if let Some(ref v) = b.text {
            q = q.bind(v);
        }
    }

    let queue = q
        .fetch_optional(&state.db.writer)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Queue '{name}' not found")))?;

    tracing::info!(queue = %name, "Queue updated");
    Ok(Json(queue))
}

pub async fn delete_queue(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Check for pending/running jobs
    let active: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM jobs WHERE queue_name = ?1 AND status IN ('pending', 'running')",
    )
    .bind(&name)
    .fetch_one(&state.db.reader)
    .await?;

    if active.0 > 0 {
        return Err(ApiError::Conflict(format!(
            "Cannot delete queue '{name}': {count} active jobs remain",
            count = active.0
        )));
    }

    let result = sqlx::query("DELETE FROM queues WHERE name = ?1")
        .bind(&name)
        .execute(&state.db.writer)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Queue '{name}' not found")));
    }

    tracing::info!(queue = %name, "Queue deleted");
    Ok(Json(serde_json::json!({"status": "deleted", "name": name})))
}

pub async fn pause_queue(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Queue>, ApiError> {
    let queue: Queue = sqlx::query_as(
        r#"UPDATE queues
           SET paused = 1,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE name = ?1
           RETURNING *"#,
    )
    .bind(&name)
    .fetch_optional(&state.db.writer)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Queue '{name}' not found")))?;

    tracing::info!(queue = %name, "Queue paused");
    Ok(Json(queue))
}

pub async fn resume_queue(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Queue>, ApiError> {
    let queue: Queue = sqlx::query_as(
        r#"UPDATE queues
           SET paused = 0,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE name = ?1
           RETURNING *"#,
    )
    .bind(&name)
    .fetch_optional(&state.db.writer)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Queue '{name}' not found")))?;

    tracing::info!(queue = %name, "Queue resumed");
    Ok(Json(queue))
}

#[derive(sqlx::FromRow)]
struct QueueStatsRow {
    name: String,
    max_concurrency: i32,
    paused: i32,
    pending: i64,
    running: i64,
    completed: i64,
    dead: i64,
}
