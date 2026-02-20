mod api;
mod config;
mod db;
mod models;
mod telemetry;
mod worker;

use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;

use config::{load_config, Cli, Config};
use db::DbPools;
use worker::engine::WorkerEngine;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPools,
    pub config: Config,
    pub start_time: Arc<Instant>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = load_config(&cli)?;

    telemetry::init_tracing(&config.logging.level, &config.logging.format);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %config.server.host,
        port = config.server.port,
        db = %config.server.db_path,
        "Starting mininq"
    );

    // Connect to database
    let db = DbPools::connect(&config.server.db_path).await?;

    // Run migrations
    db::migrations::run_migrations(&db.writer).await?;

    // Build app state
    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        start_time: Arc::new(Instant::now()),
    };

    // Build HTTP router
    let app = api::build_router(state);

    // Set up graceful shutdown
    let cancel = CancellationToken::new();

    // Start worker engine
    let http_client = reqwest::Client::new();
    let engine = Arc::new(WorkerEngine::new(
        db.clone(),
        http_client,
        config.clone(),
        cancel.clone(),
    ));

    let engine_handle = {
        let engine = engine.clone();
        tokio::spawn(async move {
            engine.run().await;
        })
    };

    // Start reaper
    let reaper_handle = tokio::spawn(worker::reaper::run_reaper(
        db.writer.clone(),
        config.worker.reaper_interval_secs,
        cancel.clone(),
    ));

    // Start scheduler
    let scheduler_handle = tokio::spawn(worker::scheduler::run_scheduler(
        db.writer.clone(),
        db.reader.clone(),
        config.worker.scheduler_interval_secs,
        cancel.clone(),
    ));

    // Start cleanup (only if retention is configured)
    let cleanup_handle = config.worker.retention_days.map(|days| {
        tokio::spawn(worker::cleanup::run_cleanup(
            db.writer.clone(),
            days,
            config.worker.cleanup_interval_secs,
            cancel.clone(),
        ))
    });

    // Start HTTP server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    info!(addr = %addr, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(cancel.clone()))
        .await?;

    // Wait for background tasks to finish
    info!("Waiting for background tasks to finish...");
    let _ = tokio::join!(engine_handle, reaper_handle, scheduler_handle);
    if let Some(handle) = cleanup_handle {
        let _ = handle.await;
    }

    // Close database
    db.close().await;
    info!("mininq shut down gracefully");

    Ok(())
}

async fn shutdown_signal(cancel: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
    cancel.cancel();
}
