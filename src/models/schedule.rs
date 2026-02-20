use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Schedule {
    pub id: String,
    pub queue_name: String,
    pub cron_expression: String,
    pub callback_url: String,
    pub payload: String,
    pub max_retries: i32,
    pub timeout_ms: i32,
    pub enabled: i32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub queue_name: Option<String>,
    pub cron_expression: String,
    pub callback_url: String,
    pub payload: Option<serde_json::Value>,
    pub max_retries: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScheduleRequest {
    pub queue_name: Option<String>,
    pub cron_expression: Option<String>,
    pub callback_url: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub max_retries: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub enabled: Option<bool>,
}
