# Configuration

mininq is configured via a TOML config file, CLI flags, and environment variables.

**Precedence:** CLI flags > environment variables > config file > defaults

## Config File

By default, mininq looks for `mininq.toml` in the current directory. Specify a different path with `--config`:

```bash
mininq --config /etc/mininq/config.toml
```

### Full Example

```toml
[server]
host = "0.0.0.0"
port = 6390
db_path = "mininq.db"

[worker]
concurrency = 10
poll_interval_ms = 500
queues = []                  # empty = all queues
reaper_interval_secs = 30
scheduler_interval_secs = 15
retention_days = 30          # omit to keep jobs forever
cleanup_interval_secs = 3600

[defaults]
max_retries = 3
retry_backoff = "exponential"  # "exponential" | "linear" | "fixed"
base_delay_ms = 1000
max_delay_ms = 300000          # 5 minutes
timeout_ms = 30000             # 30 seconds

[logging]
level = "info"
format = "json"                # "json" | "pretty"
```

## Reference

### `[server]`

| Field     | Type   | Default      | Description                          |
|-----------|--------|--------------|--------------------------------------|
| `host`    | String | `"0.0.0.0"`  | Bind address                         |
| `port`    | u16    | `6390`       | HTTP port                            |
| `db_path` | String | `"mininq.db"`| Path to the SQLite database file     |

### `[worker]`

| Field                   | Type      | Default | Description                                                      |
|-------------------------|-----------|---------|------------------------------------------------------------------|
| `concurrency`           | usize     | `10`    | Maximum concurrent job executions (semaphore permits)             |
| `poll_interval_ms`      | u64       | `500`   | Milliseconds between poll cycles                                 |
| `queues`                | [String]  | `[]`    | Queue names to process; empty means all queues                   |
| `reaper_interval_secs`  | u64       | `30`    | Seconds between reaper sweeps for stale jobs                     |
| `scheduler_interval_secs`| u64      | `15`    | Seconds between scheduler ticks for cron jobs                    |
| `retention_days`        | u32?      | _none_  | Days to keep completed/dead jobs; omit to keep forever           |
| `cleanup_interval_secs` | u64       | `3600`  | Seconds between cleanup sweeps (only active if retention is set) |

### `[defaults]`

These defaults apply to jobs that don't specify their own values.

| Field           | Type   | Default          | Description                              |
|-----------------|--------|------------------|------------------------------------------|
| `max_retries`   | i32    | `3`              | Maximum retry attempts                   |
| `retry_backoff` | String | `"exponential"`  | Backoff strategy: exponential, linear, fixed |
| `base_delay_ms` | i32    | `1000`           | Base delay for retry calculation (ms)    |
| `max_delay_ms`  | i32    | `300000`         | Maximum retry delay cap (ms)             |
| `timeout_ms`    | i32    | `30000`          | Webhook request timeout (ms)             |

### `[logging]`

| Field    | Type   | Default  | Description                       |
|----------|--------|----------|-----------------------------------|
| `level`  | String | `"info"` | Log level: trace, debug, info, warn, error |
| `format` | String | `"json"` | Output format: json or pretty     |

## CLI Flags

```
mininq [OPTIONS]

Options:
  -c, --config <PATH>       Path to config file [default: mininq.toml]
      --host <HOST>         Server host (overrides config)
      --port <PORT>         Server port (overrides config)
      --db-path <PATH>      Database path (overrides config)
      --log-level <LEVEL>   Log level (overrides config)
  -h, --help                Print help
```

## Environment Variables

| Variable          | Overrides           |
|-------------------|---------------------|
| `MININQ_HOST`     | `server.host`       |
| `MININQ_PORT`     | `server.port`       |
| `MININQ_DB_PATH`  | `server.db_path`    |
| `MININQ_LOG_LEVEL`| `logging.level`     |
