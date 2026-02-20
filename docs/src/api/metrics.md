# Metrics API

## GET /metrics

Returns aggregate system metrics.

```bash
curl http://localhost:6390/metrics
```

**Response:** `200 OK`

```json
{
  "uptime_secs": 3600,
  "version": "0.1.0",
  "jobs": {
    "total": 1500,
    "pending": 42,
    "running": 8,
    "completed": 1420,
    "dead": 30
  },
  "queues": [
    {
      "name": "default",
      "paused": false,
      "depth": 30,
      "in_flight": 5
    },
    {
      "name": "emails",
      "paused": false,
      "depth": 12,
      "in_flight": 3
    }
  ],
  "schedules": {
    "total": 5,
    "enabled": 4
  }
}
```

## Field Descriptions

| Field                | Description                                      |
|----------------------|--------------------------------------------------|
| `uptime_secs`        | Seconds since the server started                 |
| `version`            | mininq version from `Cargo.toml`                 |
| `jobs.total`         | Total job count across all statuses              |
| `jobs.pending`       | Jobs waiting to be picked up                     |
| `jobs.running`       | Jobs currently being executed                    |
| `jobs.completed`     | Successfully completed jobs                      |
| `jobs.dead`          | Jobs that exhausted retries or hit permanent failure |
| `queues[].name`      | Queue name                                       |
| `queues[].paused`    | Whether the queue is paused                      |
| `queues[].depth`     | Number of pending jobs in this queue             |
| `queues[].in_flight` | Number of currently running jobs in this queue   |
| `schedules.total`    | Total number of schedules                        |
| `schedules.enabled`  | Number of enabled (active) schedules             |
