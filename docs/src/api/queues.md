# Queues API

Queues group jobs and control concurrency, rate limiting, and retry behavior. Queues are automatically created when a job references a non-existent queue name, but you can also create them explicitly to configure settings.

---

## POST /queues

Create a queue with custom settings.

**Request body:**

| Field              | Type    | Required | Default          | Description                          |
|--------------------|---------|----------|------------------|--------------------------------------|
| `name`             | String  | yes      |                  | Queue name (unique)                  |
| `max_concurrency`  | i32     | no       | `5`              | Max concurrent jobs from this queue  |
| `rate_limit_rps`   | f64     | no       | _none_           | Requests per second limit            |
| `max_retries`      | i32     | no       | `3`              | Default max retries for jobs         |
| `retry_backoff`    | String  | no       | `"exponential"`  | Default backoff strategy             |
| `base_delay_ms`    | i32     | no       | `1000`           | Default base delay (ms)              |
| `max_delay_ms`     | i32     | no       | `300000`         | Default max delay (ms)               |

**Example:**

```bash
curl -X POST http://localhost:6390/queues \
  -H "Content-Type: application/json" \
  -d '{
    "name": "webhooks",
    "max_concurrency": 20,
    "rate_limit_rps": 10.0,
    "retry_backoff": "linear"
  }'
```

**Response:** `201 Created`

```json
{
  "name": "webhooks",
  "max_concurrency": 20,
  "rate_limit_rps": 10.0,
  "max_retries": 3,
  "retry_backoff": "linear",
  "base_delay_ms": 1000,
  "max_delay_ms": 300000,
  "paused": 0,
  "created_at": "2024-01-15T10:00:00.000",
  "updated_at": "2024-01-15T10:00:00.000"
}
```

Returns `409` if a queue with that name already exists.

---

## GET /queues

List all queues with job count statistics.

```bash
curl http://localhost:6390/queues
```

**Response:** `200 OK`

```json
[
  {
    "name": "default",
    "max_concurrency": 5,
    "paused": false,
    "pending": 12,
    "running": 3,
    "completed": 150,
    "dead": 2
  }
]
```

---

## GET /queues/{name}

Get a single queue with stats.

```bash
curl http://localhost:6390/queues/webhooks
```

**Response:** `200 OK` — queue stats object. `404` if not found.

---

## PUT /queues/{name}

Update queue settings. Only provided fields are changed.

**Request body:**

| Field              | Type    | Description                          |
|--------------------|---------|--------------------------------------|
| `max_concurrency`  | i32     | Max concurrent jobs                  |
| `rate_limit_rps`   | f64     | Requests per second limit            |
| `max_retries`      | i32     | Default max retries                  |
| `retry_backoff`    | String  | Default backoff strategy             |
| `base_delay_ms`    | i32     | Default base delay (ms)              |
| `max_delay_ms`     | i32     | Default max delay (ms)               |

**Example:**

```bash
curl -X PUT http://localhost:6390/queues/webhooks \
  -H "Content-Type: application/json" \
  -d '{"rate_limit_rps": 5.0}'
```

**Response:** `200 OK` — updated queue object. `404` if not found. `400` if no fields provided.

---

## DELETE /queues/{name}

Delete a queue. Fails if the queue has pending or running jobs.

```bash
curl -X DELETE http://localhost:6390/queues/webhooks
```

**Response:** `200 OK`

```json
{
  "status": "deleted",
  "name": "webhooks"
}
```

Returns `404` if not found. Returns `409` if the queue still has active (pending/running) jobs.

---

## POST /queues/{name}/pause

Pause a queue. Paused queues stop having their jobs picked up by workers.

```bash
curl -X POST http://localhost:6390/queues/webhooks/pause
```

**Response:** `200 OK` — updated queue object with `paused: 1`.

---

## POST /queues/{name}/resume

Resume a paused queue.

```bash
curl -X POST http://localhost:6390/queues/webhooks/resume
```

**Response:** `200 OK` — updated queue object with `paused: 0`.
