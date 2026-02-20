# Architecture

This page describes the internal design of mininq.

## Database

mininq uses SQLite in **WAL (Write-Ahead Logging) mode**, enabling concurrent reads while a write is in progress.

**Connection pools:**
- **Writer pool** — exactly 1 connection (serializes all writes)
- **Reader pool** — N connections (N = number of CPU cores, minimum 4)

**Pragmas applied to both pools:**

| Pragma            | Value           | Purpose                                  |
|-------------------|-----------------|------------------------------------------|
| `journal_mode`    | `wal`           | Concurrent read/write                    |
| `synchronous`     | `normal`        | Balance durability with performance      |
| `busy_timeout`    | `5000` ms       | Wait on lock instead of failing          |
| `temp_store`      | `memory`        | Store temp tables in RAM                 |
| `mmap_size`       | `30000000000`   | Memory-map up to ~30 GB of the DB file   |
| `cache_size`      | `-64000`        | 64 MB page cache                         |
| `foreign_keys`    | `on`            | Enforce referential integrity            |

### Schema

**3 tables:**

- **`queues`** — queue configuration (name, concurrency, rate limit, retry settings, paused flag)
- **`jobs`** — individual jobs (status, payload, callback URL, retry state, timestamps)
- **`schedules`** — cron schedules (expression, callback, next/last run)

**4 indexes:**

| Index                    | Purpose                                          |
|--------------------------|--------------------------------------------------|
| `idx_jobs_poll`          | Fast job claiming: `(queue_name, status, visible_at, priority DESC, created_at ASC) WHERE status = 'pending'` |
| `idx_jobs_reaper`        | Stale job detection: `(status, visible_at) WHERE status = 'running'` |
| `idx_jobs_idempotency`   | Idempotency key lookup: unique on `(idempotency_key) WHERE idempotency_key IS NOT NULL` |
| `idx_schedules_next_run` | Due schedule lookup: `(enabled, next_run_at) WHERE enabled = 1` |

## Worker Engine

The worker engine is a poll-based loop:

1. **Sleep** for `poll_interval_ms` (default 500ms)
2. **Acquire** a semaphore permit (limits concurrency to `worker.concurrency`)
3. **Claim** a job using an atomic `UPDATE ... WHERE id = (SELECT ...) RETURNING *` query
4. **Spawn** a tokio task to execute the webhook
5. Release the semaphore permit when the task completes

The claim query selects the highest-priority pending job whose `visible_at` has passed, atomically setting it to `running` and assigning a `worker_id`. This prevents double-execution even with multiple workers.

Queue order is randomized on each poll cycle to prevent starvation.

### Graceful Shutdown

On `SIGTERM` or `SIGINT`:
1. The `CancellationToken` is triggered
2. The worker engine stops polling for new jobs
3. It acquires all semaphore permits (blocking until in-flight jobs complete)
4. The reaper, scheduler, and cleanup tasks exit their loops
5. Database connections are closed

## Rate Limiter

Per-queue rate limiting uses a **token bucket** algorithm:

- Each queue with `rate_limit_rps` set gets its own bucket
- Bucket capacity = `rate_limit_rps` (minimum 1.0)
- Tokens refill continuously at `rate_limit_rps` per second
- Each job claim consumes 1 token
- If the bucket is empty, the queue is skipped for that poll cycle
- Queues without a rate limit always pass

Rate limits are checked from the database on each poll, so changes via `PUT /queues/{name}` take effect immediately.

## Retry Strategies

When a job fails with a transient error (5xx, timeout, connection error), it's retried up to `max_retries` times. The delay before the next attempt is computed as:

| Strategy       | Formula                              |
|----------------|--------------------------------------|
| `exponential`  | `base_delay_ms * 2^(attempt - 1)`   |
| `linear`       | `base_delay_ms * attempt`            |
| `fixed`        | `base_delay_ms`                      |

All strategies add **±30% random jitter** and are capped at `max_delay_ms`.

**Permanent failures** (4xx responses) bypass retries and go directly to `dead`.

## Reaper

The reaper runs on a configurable interval (default 30s) and recovers jobs stuck in `running` status. A job is considered stale when its `visible_at` has passed — this timestamp is set to `now + timeout_ms` when the job is claimed.

Stale jobs are reset to `pending` with `worker_id` and `started_at` cleared, making them available for re-execution.

## Scheduler

The scheduler runs on a configurable interval (default 15s) and:

1. Queries enabled schedules where `next_run_at <= now`
2. For each due schedule, computes the next run time from the cron expression
3. Uses a **CAS (Compare-And-Swap)** update: `UPDATE ... WHERE id = ? AND next_run_at = ?`
   - This prevents duplicate job creation if multiple instances are running
4. Inserts a new job with the schedule's callback URL, payload, and retry settings
5. Auto-creates the target queue if it doesn't exist

## Cleanup

If `worker.retention_days` is configured, the cleanup task runs periodically (default every 3600s) and deletes completed/dead jobs older than the retention period. Jobs in `pending` or `running` status are never deleted.
