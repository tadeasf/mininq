# Deployment

## Build a Release Binary

```bash
cargo build --release
```

The binary is at `./target/release/mininq` (~5-10 MB, statically includes the dashboard).

## Systemd

Create `/etc/systemd/system/mininq.service`:

```ini
[Unit]
Description=mininq job runner
After=network.target

[Service]
Type=simple
User=mininq
Group=mininq
WorkingDirectory=/var/lib/mininq
ExecStart=/usr/local/bin/mininq --config /etc/mininq/config.toml
Restart=on-failure
RestartSec=5

# Graceful shutdown
KillSignal=SIGTERM
TimeoutStopSec=30

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/mininq

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now mininq
```

## Docker

```dockerfile
FROM rust:1.85-slim AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/mininq /usr/local/bin/mininq

WORKDIR /data
EXPOSE 6390
CMD ["mininq"]
```

```bash
docker build -t mininq .
docker run -d -p 6390:6390 -v mininq-data:/data mininq
```

The database file (`mininq.db`) is created in `/data` — mount a volume to persist it.

## Reverse Proxy

### Nginx

```nginx
upstream mininq {
    server 127.0.0.1:6390;
}

server {
    listen 80;
    server_name jobs.example.com;

    location / {
        proxy_pass http://mininq;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Caddy

```
jobs.example.com {
    reverse_proxy localhost:6390
}
```

## Graceful Shutdown

mininq handles `SIGTERM` and `SIGINT`:

1. Stops accepting new HTTP connections
2. Waits for in-flight jobs to complete (up to their individual timeouts)
3. Stops background tasks (reaper, scheduler, cleanup)
4. Closes database connections

Set `TimeoutStopSec` in systemd or `stop_grace_period` in Docker Compose to match your longest expected job timeout.

## Backup

mininq stores everything in a single SQLite file. Back it up safely while the server is running:

```bash
sqlite3 /var/lib/mininq/mininq.db ".backup /backups/mininq-$(date +%F).db"
```

The `.backup` command uses SQLite's online backup API, which is safe to run against a live WAL-mode database.
