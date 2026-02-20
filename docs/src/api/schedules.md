# Schedules API

Schedules create jobs automatically on a cron-based interval.

## Cron Format

mininq uses the [cron](https://crates.io/crates/cron) crate, which supports a **7-field** format:

```
sec  min  hour  day-of-month  month  day-of-week  year
```

The `year` field is optional. Common examples:

| Expression              | Meaning                      |
|-------------------------|------------------------------|
| `0 */5 * * * *`         | Every 5 minutes              |
| `0 0 * * * *`           | Every hour                   |
| `0 0 9 * * *`           | Daily at 09:00               |
| `0 30 2 * * Mon`        | Mondays at 02:30             |
| `0 0 0 1 * *`           | First day of each month      |

> **Note:** The first field is **seconds**, not minutes. `0 * * * * *` means "every minute at second 0."

---

## POST /schedules

Create a recurring schedule.

**Request body:**

| Field              | Type    | Required | Default      | Description                           |
|--------------------|---------|----------|--------------|---------------------------------------|
| `cron_expression`  | String  | yes      |              | Cron expression (see format above)    |
| `callback_url`     | String  | yes      |              | HTTP(S) URL to POST                   |
| `queue_name`       | String  | no       | `"default"`  | Target queue for generated jobs       |
| `payload`          | Object  | no       | `{}`         | JSON payload for generated jobs       |
| `max_retries`      | i32     | no       | `3`          | Max retries for generated jobs        |
| `timeout_ms`       | i32     | no       | `30000`      | Timeout for generated jobs (ms)       |
| `enabled`          | bool    | no       | `true`       | Whether the schedule is active        |

**Example:**

```bash
curl -X POST http://localhost:6390/schedules \
  -H "Content-Type: application/json" \
  -d '{
    "cron_expression": "0 */5 * * * *",
    "callback_url": "https://example.com/cleanup",
    "queue_name": "maintenance",
    "payload": {"task": "expire_sessions"}
  }'
```

**Response:** `201 Created`

```json
{
  "id": "019...",
  "queue_name": "maintenance",
  "cron_expression": "0 */5 * * * *",
  "callback_url": "https://example.com/cleanup",
  "payload": "{\"task\":\"expire_sessions\"}",
  "max_retries": 3,
  "timeout_ms": 30000,
  "enabled": 1,
  "last_run_at": null,
  "next_run_at": "2024-01-15T10:05:00.000",
  "created_at": "2024-01-15T10:00:00.000",
  "updated_at": "2024-01-15T10:00:00.000"
}
```

---

## GET /schedules

List all schedules.

```bash
curl http://localhost:6390/schedules
```

**Response:** `200 OK` — array of schedule objects, ordered by `created_at DESC`.

---

## GET /schedules/{id}

Get a single schedule.

```bash
curl http://localhost:6390/schedules/019...
```

**Response:** `200 OK` — schedule object. `404` if not found.

---

## PUT /schedules/{id}

Update a schedule. Only provided fields are changed. If `cron_expression` is updated, `next_run_at` is automatically recomputed.

**Request body:**

| Field              | Type    | Description                           |
|--------------------|---------|---------------------------------------|
| `queue_name`       | String  | Target queue                          |
| `cron_expression`  | String  | Cron expression (recomputes next run) |
| `callback_url`     | String  | Callback URL                          |
| `payload`          | Object  | JSON payload                          |
| `max_retries`      | i32     | Max retries                           |
| `timeout_ms`       | i32     | Timeout (ms)                          |
| `enabled`          | bool    | Enable/disable the schedule           |

**Example:**

```bash
curl -X PUT http://localhost:6390/schedules/019... \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

**Response:** `200 OK` — updated schedule object. `404` if not found. `400` if no fields provided.

---

## DELETE /schedules/{id}

Delete a schedule.

```bash
curl -X DELETE http://localhost:6390/schedules/019...
```

**Response:** `200 OK`

```json
{
  "status": "deleted",
  "id": "019..."
}
```

Returns `404` if not found.
