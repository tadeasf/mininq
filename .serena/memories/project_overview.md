# mininq Project Overview

## Purpose
SQLite-backed job runner — background jobs the way SQLite feels for databases.

## Tech Stack
- Rust (edition 2024), Tokio async runtime
- Axum HTTP framework, sqlx for SQLite
- Writer/reader pool pattern (WAL mode, single writer, N readers)
- CancellationToken for graceful shutdown

## Structure
- `src/main.rs` - AppState, server boot, worker/reaper spawn
- `src/config.rs` - Config structs with serde/clap, TOML config file
- `src/db/` - DbPools (writer + reader), migrations (raw SQL schema)
- `src/models/` - Job, Queue, QueueStats structs
- `src/api/` - Axum handlers (health, jobs CRUD, queues list/get), ApiError
- `src/worker/` - WorkerEngine (poll loop), executor (webhook), retry, reaper

## Commands
- `cargo build` - build
- `cargo run` - run server (default port 8090)
- `cargo clippy` - lint
- `cargo fmt` - format

## Conventions
- Snake_case everywhere, minimal comments
- `sqlx::query_as` with `FromRow` derives
- Dynamic SQL with format!() for computed expressions
- Error handling via thiserror ApiError enum
- Tracing for structured logging (json format default)
