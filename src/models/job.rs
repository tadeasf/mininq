use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Dead,
}

#[allow(dead_code)]
impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Dead => "dead",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(JobStatus::Pending),
            "running" => Some(JobStatus::Running),
            "completed" => Some(JobStatus::Completed),
            "dead" => Some(JobStatus::Dead),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Job {
    pub id: String,
    pub queue_name: String,
    pub status: String,
    pub priority: i32,
    pub payload: String,
    pub callback_url: String,
    pub visible_at: String,
    pub attempt: i32,
    pub max_retries: i32,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub worker_id: Option<String>,
    pub retry_backoff: Option<String>,
    pub base_delay_ms: Option<i32>,
    pub max_delay_ms: Option<i32>,
    pub last_error: Option<String>,
    pub result: Option<String>,
    pub idempotency_key: Option<String>,
    pub timeout_ms: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub queue_name: Option<String>,
    pub priority: Option<i32>,
    pub payload: Option<serde_json::Value>,
    pub callback_url: String,
    pub max_retries: Option<i32>,
    pub retry_backoff: Option<String>,
    pub base_delay_ms: Option<i32>,
    pub max_delay_ms: Option<i32>,
    pub idempotency_key: Option<String>,
    pub timeout_ms: Option<i32>,
    pub delay_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct JobResponse {
    #[serde(flatten)]
    pub job: Job,
}

#[derive(Debug, Deserialize)]
pub struct ListJobsQuery {
    pub queue: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
