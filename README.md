# mininq

SQLite-backed job runner — background jobs the way SQLite feels for databases.

No Redis. No RabbitMQ. Just a single binary and a single file.

## Features

- **Webhook-based** — jobs POST to a callback URL; your service handles the work
- **Priority queues** with per-queue rate limiting (token bucket)
- **Retry with backoff** — exponential, linear, or fixed strategies with jitter
- **Delayed jobs** — schedule jobs to become visible after a delay
- **Cron schedules** — recurring jobs via cron expressions
- **Idempotency keys** — deduplicate job creation
- **Stale job recovery** — reaper reclaims jobs from crashed workers
- **Embedded dashboard** — htmx-powered web UI at `/dashboard`
- **Graceful shutdown** — waits for in-flight jobs on SIGTERM
- **Zero external dependencies** — just SQLite in WAL mode

## Quickstart

```bash
# Build and install
cargo install --path .

# Run (creates mininq.db in current directory)
mininq

# Enqueue a job
curl -X POST http://localhost:6390/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "callback_url": "https://httpbin.org/post",
    "payload": {"hello": "world"}
  }'

# Open the dashboard
open http://localhost:6390/dashboard
```

## Configuration

Create a `mininq.toml` in the working directory (all fields are optional):

```toml
[server]
host = "0.0.0.0"
port = 6390
db_path = "mininq.db"

[worker]
concurrency = 10
poll_interval_ms = 500

[defaults]
max_retries = 3
retry_backoff = "exponential"
timeout_ms = 30000

[logging]
level = "info"
format = "json"
```

CLI flags and environment variables (`MININQ_HOST`, `MININQ_PORT`, `MININQ_DB_PATH`, `MININQ_LOG_LEVEL`) override the config file.

## API

| Method   | Path                    | Description                   |
|----------|-------------------------|-------------------------------|
| `POST`   | `/jobs`                 | Create a job                  |
| `GET`    | `/jobs`                 | List jobs (filterable)        |
| `GET`    | `/jobs/{id}`            | Get job details               |
| `DELETE` | `/jobs/{id}`            | Cancel a pending job          |
| `POST`   | `/jobs/{id}/retry`      | Retry a dead job              |
| `POST`   | `/queues`               | Create a queue                |
| `GET`    | `/queues`               | List queues with stats        |
| `PUT`    | `/queues/{name}`        | Update queue settings         |
| `POST`   | `/queues/{name}/pause`  | Pause a queue                 |
| `POST`   | `/queues/{name}/resume` | Resume a queue                |
| `POST`   | `/schedules`            | Create a cron schedule        |
| `GET`    | `/schedules`            | List schedules                |
| `PUT`    | `/schedules/{id}`       | Update a schedule             |
| `DELETE` | `/schedules/{id}`       | Delete a schedule             |
| `GET`    | `/metrics`              | System metrics (JSON)         |
| `GET`    | `/dashboard`            | Web dashboard                 |

## Documentation

Full documentation is available at the [docs site](https://tadeasf.github.io/mininq/).

- [Getting Started](https://tadeasf.github.io/mininq/getting-started.html)
- [Configuration Reference](https://tadeasf.github.io/mininq/configuration.html)
- [API Reference](https://tadeasf.github.io/mininq/api/overview.html)
- [Architecture](https://tadeasf.github.io/mininq/architecture.html)
- [Deployment](https://tadeasf.github.io/mininq/deployment.html)

## License

MIT
