# Introduction

**mininq** is a SQLite-backed job runner — background jobs the way SQLite feels for databases. No Redis, no RabbitMQ, no external dependencies. Just a single binary and a single file.

## Why mininq?

Most job queues require running a separate broker (Redis, RabbitMQ, Postgres). mininq takes the SQLite approach: embed the queue directly alongside your application. It's ideal for:

- Small-to-medium services that don't need a dedicated message broker
- Self-contained deployments where fewer moving parts matter
- Projects that already use SQLite and want to keep things simple

## Architecture Overview

```
┌──────────────┐     POST /jobs      ┌────────────────┐
│  Your App    │ ──────────────────▶ │   mininq HTTP  │
│              │                     │   API (Axum)   │
└──────────────┘                     └───────┬────────┘
                                             │
                                             ▼
                                     ┌───────────────┐
                                     │  SQLite (WAL)  │
                                     │   mininq.db    │
                                     └───────┬────────┘
                                             │
                                             ▼
                                     ┌───────────────┐
                                     │ Worker Engine  │──▶ POST callback_url
                                     │ (poll loop)    │    (webhook delivery)
                                     └───────────────┘
```

## Features

- **Webhook-based execution** — jobs POST to a callback URL; your service handles the work
- **Priority queues** — higher-priority jobs run first within a queue
- **Per-queue rate limiting** — token bucket algorithm, configurable per queue
- **Retry with backoff** — exponential, linear, or fixed strategies with jitter
- **Delayed jobs** — schedule jobs to become visible after a delay
- **Cron schedules** — recurring jobs via 6-field cron expressions
- **Idempotency keys** — deduplicate job creation
- **Stale job recovery** — reaper reclaims jobs from crashed workers
- **Automatic cleanup** — optionally delete completed/dead jobs after N days
- **Embedded dashboard** — htmx-powered web UI for monitoring
- **Graceful shutdown** — waits for in-flight jobs on SIGTERM/SIGINT
- **Single binary, single file** — no runtime dependencies beyond the OS
