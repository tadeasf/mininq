# Getting Started

## Prerequisites

- [Rust toolchain](https://rustup.rs/) (1.85+ for edition 2024)

## Install

```bash
# Clone and build
git clone https://github.com/tadeasf/mininq.git
cd mininq
cargo install --path .
```

Or build without installing:

```bash
cargo build --release
# Binary at ./target/release/mininq
```

## Run

```bash
mininq
```

mininq creates `mininq.db` in the current directory and starts listening on `0.0.0.0:6390`.

```
2024-01-15T10:00:00 INFO mininq: Starting mininq version=0.1.0 host=0.0.0.0 port=6390 db=mininq.db
2024-01-15T10:00:00 INFO mininq::db: Database pools connected writer_max=1 reader_max=4 path=mininq.db
2024-01-15T10:00:00 INFO mininq: HTTP server listening addr=0.0.0.0:6390
```

## Enqueue Your First Job

Use any HTTP endpoint as a callback. Here we use [httpbin.org](https://httpbin.org) as a test target:

```bash
curl -X POST http://localhost:6390/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "callback_url": "https://httpbin.org/post",
    "payload": {"message": "hello from mininq"}
  }'
```

Response:

```json
{
  "id": "019...",
  "queue_name": "default",
  "status": "pending",
  "priority": 0,
  "payload": "{\"message\":\"hello from mininq\"}",
  "callback_url": "https://httpbin.org/post",
  "attempt": 0,
  "max_retries": 3,
  "timeout_ms": 30000,
  "created_at": "2024-01-15T10:00:01.234",
  "updated_at": "2024-01-15T10:00:01.234"
}
```

## Check Job Status

```bash
curl http://localhost:6390/jobs/019...
```

The job should quickly move from `pending` → `running` → `completed`.

## Open the Dashboard

Navigate to [http://localhost:6390/dashboard](http://localhost:6390/dashboard) to see stats, queues, jobs, and schedules in a live-updating web UI.

## Next Steps

- [Configuration](./configuration.md) — customize ports, concurrency, retry behavior
- [API Reference](./api/overview.md) — full endpoint documentation
- [Architecture](./architecture.md) — understand the internals
