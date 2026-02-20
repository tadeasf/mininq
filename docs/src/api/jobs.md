# Jobs API

## Job Lifecycle

```
pending ──▶ running ──▶ completed
               │
               ▼
           (retry) ──▶ pending   (if attempts < max_retries)
               │
               ▼
             dead                (if attempts exhausted or 4xx response)
```

When a job is executed, mininq sends a POST request to the `callback_url` with:
- **Body:** the job's `payload` (JSON)
- **Headers:**
  - `Content-Type: application/json`
  - `X-Mininq-Job-Id: <job-id>`
  - `X-Mininq-Attempt: <attempt-number>`
  - `X-Mininq-Queue: <queue-name>`

**Response handling:**
- **2xx** → job marked `completed`, response body saved to `result`
- **4xx** → permanent failure, job goes directly to `dead`
- **5xx / timeout / connection error** → transient failure, retried up to `max_retries`

---

## POST /jobs

Create a new job.

**Request body:**

| Field             | Type    | Required | Default        | Description                                    |
|-------------------|---------|----------|----------------|------------------------------------------------|
| `callback_url`    | String  | yes      |                | HTTP(S) URL to POST when executing             |
| `queue_name`      | String  | no       | `"default"`    | Target queue (auto-created if missing)          |
| `priority`        | i32     | no       | `0`            | Higher values execute first                    |
| `payload`         | Object  | no       | `{}`           | JSON payload sent to the callback              |
| `max_retries`     | i32     | no       | config default | Maximum retry attempts                         |
| `retry_backoff`   | String  | no       | config default | `"exponential"`, `"linear"`, or `"fixed"`      |
| `base_delay_ms`   | i32     | no       | config default | Base delay for retry calculation               |
| `max_delay_ms`    | i32     | no       | config default | Maximum retry delay cap                        |
| `timeout_ms`      | i32     | no       | config default | Webhook request timeout (ms)                   |
| `idempotency_key` | String  | no       |                | Dedup key — returns existing job if matched    |
| `delay_ms`        | i64     | no       |                | Delay before job becomes visible (ms)          |

**Example:**

```bash
curl -X POST http://localhost:6390/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "callback_url": "https://example.com/webhook",
    "queue_name": "emails",
    "priority": 10,
    "payload": {"to": "user@example.com", "template": "welcome"},
    "max_retries": 5,
    "idempotency_key": "welcome-user-123"
  }'
```

**Response:** `201 Created` (or `200 OK` if idempotency key matched an existing job)

```json
{
  "id": "019...",
  "queue_name": "emails",
  "status": "pending",
  "priority": 10,
  "payload": "{\"to\":\"user@example.com\",\"template\":\"welcome\"}",
  "callback_url": "https://example.com/webhook",
  "visible_at": "2024-01-15T10:00:01.234",
  "attempt": 0,
  "max_retries": 5,
  "timeout_ms": 30000,
  "idempotency_key": "welcome-user-123",
  "created_at": "2024-01-15T10:00:01.234",
  "updated_at": "2024-01-15T10:00:01.234"
}
```

### Delayed Jobs

Set `delay_ms` to schedule a job for future execution:

```bash
curl -X POST http://localhost:6390/jobs \
  -H "Content-Type: application/json" \
  -d '{
    "callback_url": "https://example.com/webhook",
    "delay_ms": 60000
  }'
```

The job will remain invisible to the worker until the delay has elapsed.

---

## GET /jobs

List jobs with optional filters.

**Query parameters:**

| Param    | Type   | Default | Description                         |
|----------|--------|---------|-------------------------------------|
| `queue`  | String |         | Filter by queue name                |
| `status` | String |         | Filter by status (pending, running, completed, dead) |
| `limit`  | i64    | `50`    | Max results (capped at 1000)        |
| `offset` | i64    | `0`     | Pagination offset                   |

**Example:**

```bash
curl "http://localhost:6390/jobs?queue=emails&status=dead&limit=10"
```

**Response:** `200 OK` — array of job objects.

---

## GET /jobs/{id}

Get a single job by ID.

```bash
curl http://localhost:6390/jobs/019...
```

**Response:** `200 OK` — job object. `404` if not found.

---

## DELETE /jobs/{id}

Cancel a pending job. Only jobs with status `pending` can be cancelled.

```bash
curl -X DELETE http://localhost:6390/jobs/019...
```

**Response:** `200 OK`

```json
{
  "status": "cancelled",
  "id": "019..."
}
```

Returns `404` if not found, `409` if the job is not in `pending` status.

---

## POST /jobs/{id}/retry

Retry a dead job. Resets it to `pending` with attempt count 0.

```bash
curl -X POST http://localhost:6390/jobs/019.../retry
```

**Response:** `200 OK` — the reset job object. `404` if not found or not in `dead` status.
