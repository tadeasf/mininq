# Dashboard

mininq includes an embedded web dashboard powered by htmx. No separate frontend build is required — it's compiled into the binary via [rust-embed](https://crates.io/crates/rust-embed).

## Accessing the Dashboard

Open your browser to:

```
http://localhost:6390/dashboard
```

(Replace `localhost:6390` with your configured host and port.)

## What It Shows

The dashboard provides a live-updating overview of the system:

- **Stats cards** — total jobs, pending, running, completed, dead counts at a glance
- **Queues table** — all queues with their job counts, pause/resume controls, and concurrency settings
- **Jobs table** — filterable list of jobs with status, queue, priority, timestamps, and error messages
- **Schedules table** — all cron schedules with their expression, next run time, and enabled status

## Interactivity

- **Pause/Resume** — click buttons in the queues table to pause or resume individual queues
- **Auto-refresh** — the dashboard polls the API periodically to keep data current
- **Filtering** — filter jobs by queue name and status

## Static Assets

The dashboard's HTML and CSS are embedded into the binary at compile time from the `static/` directory. The files are served at:

- `GET /dashboard` — main HTML page
- `GET /static/{path}` — CSS and other static assets
