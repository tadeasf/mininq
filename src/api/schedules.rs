use std::str::FromStr;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use crate::api::error::ApiError;
use crate::models::schedule::{CreateScheduleRequest, Schedule, UpdateScheduleRequest};
use crate::AppState;

fn compute_next_run(cron_expr: &str) -> Result<String, ApiError> {
    let schedule = cron::Schedule::from_str(cron_expr)
        .map_err(|e| ApiError::BadRequest(format!("Invalid cron expression: {e}")))?;

    let next = schedule
        .upcoming(Utc)
        .next()
        .ok_or_else(|| ApiError::BadRequest("Cron expression has no upcoming runs".to_string()))?;

    Ok(next.format("%Y-%m-%dT%H:%M:%S%.3f").to_string())
}

pub async fn create_schedule(
    State(state): State<AppState>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<Schedule>), ApiError> {
    if req.callback_url.is_empty() {
        return Err(ApiError::BadRequest("callback_url is required".to_string()));
    }
    if !req.callback_url.starts_with("http://") && !req.callback_url.starts_with("https://") {
        return Err(ApiError::BadRequest(
            "callback_url must be an HTTP(S) URL".to_string(),
        ));
    }

    let next_run_at = compute_next_run(&req.cron_expression)?;

    let id = uuid::Uuid::now_v7().to_string();
    let queue_name = req.queue_name.as_deref().unwrap_or("default");
    let payload = req
        .payload
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let max_retries = req.max_retries.unwrap_or(3);
    let timeout_ms = req.timeout_ms.unwrap_or(30000);
    let enabled = if req.enabled.unwrap_or(true) { 1 } else { 0 };

    let schedule: Schedule = sqlx::query_as(
        r#"INSERT INTO schedules (id, queue_name, cron_expression, callback_url, payload,
                                   max_retries, timeout_ms, enabled, next_run_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
           RETURNING *"#,
    )
    .bind(&id)
    .bind(queue_name)
    .bind(&req.cron_expression)
    .bind(&req.callback_url)
    .bind(&payload)
    .bind(max_retries)
    .bind(timeout_ms)
    .bind(enabled)
    .bind(&next_run_at)
    .fetch_one(&state.db.writer)
    .await?;

    tracing::info!(schedule_id = %id, cron = %req.cron_expression, "Schedule created");
    Ok((StatusCode::CREATED, Json(schedule)))
}

pub async fn list_schedules(
    State(state): State<AppState>,
) -> Result<Json<Vec<Schedule>>, ApiError> {
    let schedules: Vec<Schedule> =
        sqlx::query_as("SELECT * FROM schedules ORDER BY created_at DESC")
            .fetch_all(&state.db.reader)
            .await?;

    Ok(Json(schedules))
}

pub async fn get_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Schedule>, ApiError> {
    let schedule: Schedule = sqlx::query_as("SELECT * FROM schedules WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&state.db.reader)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Schedule '{id}' not found")))?;

    Ok(Json(schedule))
}

pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<Schedule>, ApiError> {
    let mut sets = Vec::new();
    let mut bind_idx = 1u32;

    enum BindVal {
        Int(i32),
        Text(String),
    }
    let mut binds: Vec<BindVal> = Vec::new();

    if let Some(ref v) = req.queue_name {
        bind_idx += 1;
        sets.push(format!("queue_name = ?{bind_idx}"));
        binds.push(BindVal::Text(v.clone()));
    }
    if let Some(ref v) = req.cron_expression {
        // Validate and recompute next_run_at
        let next = compute_next_run(v)?;
        bind_idx += 1;
        sets.push(format!("cron_expression = ?{bind_idx}"));
        binds.push(BindVal::Text(v.clone()));
        bind_idx += 1;
        sets.push(format!("next_run_at = ?{bind_idx}"));
        binds.push(BindVal::Text(next));
    }
    if let Some(ref v) = req.callback_url {
        bind_idx += 1;
        sets.push(format!("callback_url = ?{bind_idx}"));
        binds.push(BindVal::Text(v.clone()));
    }
    if let Some(ref v) = req.payload {
        bind_idx += 1;
        sets.push(format!("payload = ?{bind_idx}"));
        binds.push(BindVal::Text(v.to_string()));
    }
    if let Some(v) = req.max_retries {
        bind_idx += 1;
        sets.push(format!("max_retries = ?{bind_idx}"));
        binds.push(BindVal::Int(v));
    }
    if let Some(v) = req.timeout_ms {
        bind_idx += 1;
        sets.push(format!("timeout_ms = ?{bind_idx}"));
        binds.push(BindVal::Int(v));
    }
    if let Some(v) = req.enabled {
        bind_idx += 1;
        sets.push(format!("enabled = ?{bind_idx}"));
        binds.push(BindVal::Int(if v { 1 } else { 0 }));
    }

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    sets.push("updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')".to_string());

    let query = format!(
        "UPDATE schedules SET {} WHERE id = ?1 RETURNING *",
        sets.join(", ")
    );

    let mut q = sqlx::query_as::<_, Schedule>(&query).bind(&id);
    for b in &binds {
        match b {
            BindVal::Int(v) => q = q.bind(*v),
            BindVal::Text(v) => q = q.bind(v),
        }
    }

    let schedule = q
        .fetch_optional(&state.db.writer)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Schedule '{id}' not found")))?;

    tracing::info!(schedule_id = %id, "Schedule updated");
    Ok(Json(schedule))
}

pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM schedules WHERE id = ?1")
        .bind(&id)
        .execute(&state.db.writer)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Schedule '{id}' not found")));
    }

    tracing::info!(schedule_id = %id, "Schedule deleted");
    Ok(Json(serde_json::json!({"status": "deleted", "id": id})))
}
