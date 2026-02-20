# API Overview

mininq exposes a JSON REST API over HTTP.

## Base URL

```
http://localhost:6390
```

All request and response bodies use `Content-Type: application/json`.

## Error Format

All errors return a JSON object with `error` and `status` fields:

```json
{
  "error": "Job 019... not found",
  "status": 404
}
```

## Status Codes

| Code | Meaning                                    |
|------|--------------------------------------------|
| 200  | Success                                    |
| 201  | Resource created                           |
| 400  | Bad request (validation error)             |
| 404  | Resource not found                         |
| 409  | Conflict (e.g., duplicate, active jobs)    |
| 500  | Internal server error                      |

## Route Table

| Method   | Path                   | Description                    |
|----------|------------------------|--------------------------------|
| `GET`    | `/health`              | Health check                   |
| `POST`   | `/jobs`                | Create a job                   |
| `GET`    | `/jobs`                | List jobs (with filters)       |
| `GET`    | `/jobs/{id}`           | Get a single job               |
| `DELETE` | `/jobs/{id}`           | Cancel a pending job           |
| `POST`   | `/jobs/{id}/retry`     | Retry a dead job               |
| `POST`   | `/queues`              | Create a queue                 |
| `GET`    | `/queues`              | List queues with stats         |
| `GET`    | `/queues/{name}`       | Get a single queue with stats  |
| `PUT`    | `/queues/{name}`       | Update queue settings          |
| `DELETE` | `/queues/{name}`       | Delete a queue                 |
| `POST`   | `/queues/{name}/pause` | Pause a queue                  |
| `POST`   | `/queues/{name}/resume`| Resume a paused queue          |
| `POST`   | `/schedules`           | Create a schedule              |
| `GET`    | `/schedules`           | List all schedules             |
| `GET`    | `/schedules/{id}`      | Get a single schedule          |
| `PUT`    | `/schedules/{id}`      | Update a schedule              |
| `DELETE` | `/schedules/{id}`      | Delete a schedule              |
| `GET`    | `/metrics`             | Get system metrics             |
| `GET`    | `/dashboard`           | Web dashboard (HTML)           |
