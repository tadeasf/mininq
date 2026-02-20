use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Queue {
    pub name: String,
    pub max_concurrency: i32,
    pub rate_limit_rps: Option<f64>,
    pub max_retries: i32,
    pub retry_backoff: String,
    pub base_delay_ms: i32,
    pub max_delay_ms: i32,
    pub paused: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct QueueStats {
    pub name: String,
    pub max_concurrency: i32,
    pub paused: bool,
    pub pending: i64,
    pub running: i64,
    pub completed: i64,
    pub dead: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateQueueRequest {
    pub name: String,
    pub max_concurrency: Option<i32>,
    pub rate_limit_rps: Option<f64>,
    pub max_retries: Option<i32>,
    pub retry_backoff: Option<String>,
    pub base_delay_ms: Option<i32>,
    pub max_delay_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQueueRequest {
    pub max_concurrency: Option<i32>,
    pub rate_limit_rps: Option<f64>,
    pub max_retries: Option<i32>,
    pub retry_backoff: Option<String>,
    pub base_delay_ms: Option<i32>,
    pub max_delay_ms: Option<i32>,
}
