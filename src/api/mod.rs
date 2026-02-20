pub mod dashboard;
pub mod error;
pub mod health;
pub mod jobs;
pub mod metrics;
pub mod queues;
pub mod schedules;

use axum::routing::{get, post};
use axum::Router;

use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/jobs", post(jobs::create_job).get(jobs::list_jobs))
        .route(
            "/jobs/{id}",
            get(jobs::get_job).delete(jobs::cancel_job),
        )
        .route("/jobs/{id}/retry", post(jobs::retry_job))
        .route(
            "/queues",
            post(queues::create_queue).get(queues::list_queues),
        )
        .route(
            "/queues/{name}",
            get(queues::get_queue)
                .put(queues::update_queue)
                .delete(queues::delete_queue),
        )
        .route("/queues/{name}/pause", post(queues::pause_queue))
        .route("/queues/{name}/resume", post(queues::resume_queue))
        .route(
            "/schedules",
            post(schedules::create_schedule).get(schedules::list_schedules),
        )
        .route(
            "/schedules/{id}",
            get(schedules::get_schedule)
                .put(schedules::update_schedule)
                .delete(schedules::delete_schedule),
        )
        .route("/metrics", get(metrics::get_metrics))
        .route("/dashboard", get(dashboard::dashboard))
        .route("/static/{*path}", get(dashboard::static_file))
        .with_state(state)
}
