use std::time::Duration;

use crate::models::job::Job;

#[derive(Debug)]
pub enum WebhookResult {
    Success(Option<String>),
    PermanentFailure(String),
    TransientFailure(String),
    Timeout,
    ConnectionError(String),
}

pub async fn execute_webhook(
    client: &reqwest::Client,
    job: &Job,
) -> WebhookResult {
    let timeout = Duration::from_millis(job.timeout_ms as u64);

    let result = client
        .post(&job.callback_url)
        .header("X-Mininq-Job-Id", &job.id)
        .header("X-Mininq-Attempt", job.attempt.to_string())
        .header("X-Mininq-Queue", &job.queue_name)
        .header("Content-Type", "application/json")
        .body(job.payload.clone())
        .timeout(timeout)
        .send()
        .await;

    match result {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.is_success() {
                WebhookResult::Success(if body.is_empty() {
                    None
                } else {
                    Some(body)
                })
            } else if status.is_client_error() {
                WebhookResult::PermanentFailure(format!(
                    "HTTP {status}: {body}"
                ))
            } else {
                WebhookResult::TransientFailure(format!(
                    "HTTP {status}: {body}"
                ))
            }
        }
        Err(e) if e.is_timeout() => WebhookResult::Timeout,
        Err(e) if e.is_connect() => {
            WebhookResult::ConnectionError(e.to_string())
        }
        Err(e) => WebhookResult::TransientFailure(e.to_string()),
    }
}
