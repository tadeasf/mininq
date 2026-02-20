use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::models::job::{CreateJobRequest, Job, JobResponse, ListJobsQuery};
use crate::AppState;

pub async fn create_job(
    State(state): State<AppState>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<JobResponse>), ApiError> {
    // Validate callback_url
    if req.callback_url.is_empty() {
        return Err(ApiError::BadRequest("callback_url is required".to_string()));
    }
    if !req.callback_url.starts_with("http://") && !req.callback_url.starts_with("https://") {
        return Err(ApiError::BadRequest(
            "callback_url must be an HTTP(S) URL".to_string(),
        ));
    }

    // Handle idempotency key
    if let Some(ref key) = req.idempotency_key {
        let existing: Option<Job> =
            sqlx::query_as("SELECT * FROM jobs WHERE idempotency_key = ?1")
                .bind(key)
                .fetch_optional(&state.db.reader)
                .await?;

        if let Some(job) = existing {
            return Ok((StatusCode::OK, Json(JobResponse { job })));
        }
    }

    let id = Uuid::now_v7().to_string();
    let queue_name = req.queue_name.as_deref().unwrap_or("default");
    let priority = req.priority.unwrap_or(0);
    let payload = req
        .payload
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let max_retries = req.max_retries.unwrap_or(state.config.defaults.max_retries);
    let timeout_ms = req.timeout_ms.unwrap_or(state.config.defaults.timeout_ms);

    // Compute visible_at for delayed jobs
    let visible_at_expr = if let Some(delay_ms) = req.delay_ms {
        if delay_ms < 0 {
            return Err(ApiError::BadRequest(
                "delay_ms must be non-negative".to_string(),
            ));
        }
        let delay_secs = delay_ms as f64 / 1000.0;
        format!(
            "strftime('%Y-%m-%dT%H:%M:%f', 'now', '+{delay_secs} seconds')"
        )
    } else {
        "strftime('%Y-%m-%dT%H:%M:%f', 'now')".to_string()
    };

    // Ensure queue exists (upsert with defaults)
    sqlx::query("INSERT OR IGNORE INTO queues (name) VALUES (?1)")
        .bind(queue_name)
        .execute(&state.db.writer)
        .await?;

    let query = format!(
        r#"INSERT INTO jobs (id, queue_name, priority, payload, callback_url, max_retries,
                             retry_backoff, base_delay_ms, max_delay_ms, idempotency_key,
                             timeout_ms, visible_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, {visible_at_expr})
           RETURNING *"#
    );

    let job: Job = sqlx::query_as(&query)
        .bind(&id)
        .bind(queue_name)
        .bind(priority)
        .bind(&payload)
        .bind(&req.callback_url)
        .bind(max_retries)
        .bind(req.retry_backoff.as_deref())
        .bind(req.base_delay_ms)
        .bind(req.max_delay_ms)
        .bind(req.idempotency_key.as_deref())
        .bind(timeout_ms)
        .fetch_one(&state.db.writer)
        .await?;

    tracing::info!(job_id = %job.id, queue = %job.queue_name, "Job enqueued");
    Ok((StatusCode::CREATED, Json(JobResponse { job })))
}

pub async fn list_jobs(
    State(state): State<AppState>,
    Query(params): Query<ListJobsQuery>,
) -> Result<Json<Vec<Job>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0);

    let mut query = String::from("SELECT * FROM jobs WHERE 1=1");
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref queue) = params.queue {
        binds.push(queue.clone());
        query.push_str(&format!(" AND queue_name = ?{}", binds.len()));
    }
    if let Some(ref status) = params.status {
        binds.push(status.clone());
        query.push_str(&format!(" AND status = ?{}", binds.len()));
    }

    query.push_str(" ORDER BY created_at DESC");
    query.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

    let mut q = sqlx::query_as::<_, Job>(&query);
    for bind in &binds {
        q = q.bind(bind);
    }

    let jobs = q.fetch_all(&state.db.reader).await?;
    Ok(Json(jobs))
}

pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobResponse>, ApiError> {
    let job: Job = sqlx::query_as("SELECT * FROM jobs WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&state.db.reader)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Job {id} not found")))?;

    Ok(Json(JobResponse { job }))
}

pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "DELETE FROM jobs WHERE id = ?1 AND status = 'pending'"
    )
    .bind(&id)
    .execute(&state.db.writer)
    .await?;

    if result.rows_affected() == 0 {
        // Check if job exists
        let exists: Option<(String,)> =
            sqlx::query_as("SELECT status FROM jobs WHERE id = ?1")
                .bind(&id)
                .fetch_optional(&state.db.reader)
                .await?;

        match exists {
            None => return Err(ApiError::NotFound(format!("Job {id} not found"))),
            Some((status,)) => {
                return Err(ApiError::Conflict(format!(
                    "Cannot cancel job with status '{status}'"
                )));
            }
        }
    }

    tracing::info!(job_id = %id, "Job cancelled");
    Ok(Json(json!({"status": "cancelled", "id": id})))
}

pub async fn retry_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobResponse>, ApiError> {
    let job: Job = sqlx::query_as(
        r#"UPDATE jobs
           SET status = 'pending',
               attempt = 0,
               visible_at = strftime('%Y-%m-%dT%H:%M:%f', 'now'),
               last_error = NULL,
               worker_id = NULL,
               started_at = NULL,
               completed_at = NULL,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
           WHERE id = ?1 AND status = 'dead'
           RETURNING *"#,
    )
    .bind(&id)
    .fetch_optional(&state.db.writer)
    .await?
    .ok_or_else(|| {
        ApiError::NotFound(format!(
            "Job {id} not found or not in 'dead' status"
        ))
    })?;

    tracing::info!(job_id = %job.id, "Dead job retried");
    Ok(Json(JobResponse { job }))
}
