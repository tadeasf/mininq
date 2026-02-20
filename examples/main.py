"""
mininq example backend — FastAPI + httpx

Callback receivers (webhook targets) + demo client that exercises every mininq API endpoint.

Dependencies: pip install fastapi uvicorn httpx
Run:          python examples/main.py
              (mininq must be running on port 6390)
"""

import asyncio
from contextlib import asynccontextmanager

import httpx
import uvicorn
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

MININQ = "http://localhost:6390"
SELF = "http://localhost:8000"

# Track first-attempt per job for the flaky endpoint
_seen_jobs: set[str] = set()


@asynccontextmanager
async def lifespan(_app: FastAPI):
    print(f"\n  Example server ready on {SELF}")
    print(f"  mininq expected at      {MININQ}")
    print(f"  POST {SELF}/demo/tour to run the full feature tour\n")
    yield


app = FastAPI(title="mininq example", lifespan=lifespan)


# ---------------------------------------------------------------------------
# Section 1: Callback Receivers
# ---------------------------------------------------------------------------


@app.post("/webhook/generic")
async def webhook_generic(request: Request):
    body = await request.json()
    job_id = request.headers.get("x-mininq-job-id", "?")
    attempt = request.headers.get("x-mininq-attempt", "?")
    queue = request.headers.get("x-mininq-queue", "?")
    print(f"[generic]  job={job_id} attempt={attempt} queue={queue} payload={body}")
    return {"received": True}


@app.post("/webhook/slow")
async def webhook_slow(request: Request):
    await request.json()
    job_id = request.headers.get("x-mininq-job-id", "?")
    print(f"[slow]     job={job_id} sleeping 2s...")
    await asyncio.sleep(2)
    print(f"[slow]     job={job_id} done")
    return {"received": True, "delayed": True}


@app.post("/webhook/flaky")
async def webhook_flaky(request: Request):
    await request.json()
    job_id = request.headers.get("x-mininq-job-id", "unknown")
    attempt = request.headers.get("x-mininq-attempt", "?")
    if job_id not in _seen_jobs:
        _seen_jobs.add(job_id)
        print(f"[flaky]    job={job_id} attempt={attempt} -> 500 (first try)")
        return JSONResponse({"error": "flaky failure"}, status_code=500)
    print(f"[flaky]    job={job_id} attempt={attempt} -> 200 (retry OK)")
    return {"received": True, "retried": True}


@app.post("/webhook/always-fail")
async def webhook_always_fail(request: Request):
    await request.json()
    job_id = request.headers.get("x-mininq-job-id", "?")
    attempt = request.headers.get("x-mininq-attempt", "?")
    print(f"[fail]     job={job_id} attempt={attempt} -> 500")
    return JSONResponse({"error": "always fails"}, status_code=500)


@app.post("/webhook/permanent-fail")
async def webhook_permanent_fail(request: Request):
    await request.json()
    job_id = request.headers.get("x-mininq-job-id", "?")
    print(f"[perm-fail] job={job_id} -> 400 (immediate dead)")
    return JSONResponse({"error": "bad request, no retry"}, status_code=400)


# ---------------------------------------------------------------------------
# Section 2: Demo Endpoints (individual feature demos)
# ---------------------------------------------------------------------------

async def _fwd(method: str, path: str, **kwargs) -> dict:
    """Forward a request to mininq and return the JSON response."""
    async with httpx.AsyncClient(base_url=MININQ, timeout=30) as c:
        r = await getattr(c, method)(path, **kwargs)
        try:
            data = r.json()
        except Exception:
            data = r.text
        return {"status": r.status_code, "body": data}


# Queue management

@app.post("/demo/create-queue")
async def demo_create_queue():
    return await _fwd("post", "/queues", json={
        "name": "demo-queue",
        "max_concurrency": 2,
        "rate_limit_rps": 5.0,
        "retry_backoff": "linear",
        "base_delay_ms": 1000,
        "max_delay_ms": 10000,
    })

@app.get("/demo/list-queues")
async def demo_list_queues():
    return await _fwd("get", "/queues")

@app.put("/demo/update-queue")
async def demo_update_queue():
    return await _fwd("put", "/queues/demo-queue", json={"max_concurrency": 4})

@app.post("/demo/pause-queue")
async def demo_pause_queue():
    return await _fwd("post", "/queues/demo-queue/pause")

@app.post("/demo/resume-queue")
async def demo_resume_queue():
    return await _fwd("post", "/queues/demo-queue/resume")

@app.delete("/demo/delete-queue")
async def demo_delete_queue():
    return await _fwd("delete", "/queues/demo-queue")


# Job lifecycle

@app.post("/demo/basic-job")
async def demo_basic_job():
    return await _fwd("post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"msg": "hello from demo"},
    })

@app.post("/demo/priority-jobs")
async def demo_priority_jobs():
    results = []
    for p in [1, 5, 10]:
        r = await _fwd("post", "/jobs", json={
            "queue_name": "demo-queue",
            "callback_url": f"{SELF}/webhook/generic",
            "payload": {"priority_test": True, "priority": p},
            "priority": p,
        })
        results.append(r)
    return results

@app.post("/demo/delayed-job")
async def demo_delayed_job():
    return await _fwd("post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"delayed": True},
        "delay_ms": 5000,
    })

@app.post("/demo/idempotent-job")
async def demo_idempotent_job():
    body = {
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"idempotent": True},
        "idempotency_key": "demo-idem-key-001",
    }
    first = await _fwd("post", "/jobs", json=body)
    second = await _fwd("post", "/jobs", json=body)
    return {"first": first, "second": second}

@app.post("/demo/retry-job")
async def demo_retry_job():
    return await _fwd("post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/flaky",
        "payload": {"flaky": True},
        "max_retries": 3,
        "retry_backoff": "fixed",
        "base_delay_ms": 500,
    })

@app.post("/demo/failing-job")
async def demo_failing_job():
    always = await _fwd("post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/always-fail",
        "payload": {"will_die": True},
        "max_retries": 1,
        "retry_backoff": "fixed",
        "base_delay_ms": 500,
    })
    permanent = await _fwd("post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/permanent-fail",
        "payload": {"permanent_fail": True},
    })
    return {"always_fail": always, "permanent_fail": permanent}

@app.get("/demo/list-jobs")
async def demo_list_jobs():
    return await _fwd("get", "/jobs?queue=demo-queue&limit=20")

@app.get("/demo/get-job/{job_id}")
async def demo_get_job(job_id: str):
    return await _fwd("get", f"/jobs/{job_id}")

@app.delete("/demo/cancel-job/{job_id}")
async def demo_cancel_job(job_id: str):
    return await _fwd("delete", f"/jobs/{job_id}")

@app.post("/demo/retry-dead-job/{job_id}")
async def demo_retry_dead_job(job_id: str):
    return await _fwd("post", f"/jobs/{job_id}/retry")


# Schedules

@app.post("/demo/create-schedule")
async def demo_create_schedule():
    return await _fwd("post", "/schedules", json={
        "queue_name": "demo-queue",
        "cron_expression": "0 * * * * *",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"scheduled": True},
    })

@app.get("/demo/list-schedules")
async def demo_list_schedules():
    return await _fwd("get", "/schedules")

@app.put("/demo/update-schedule/{schedule_id}")
async def demo_update_schedule(schedule_id: str):
    return await _fwd("put", f"/schedules/{schedule_id}", json={"enabled": False})

@app.delete("/demo/delete-schedule/{schedule_id}")
async def demo_delete_schedule(schedule_id: str):
    return await _fwd("delete", f"/schedules/{schedule_id}")


# Observability

@app.get("/demo/health")
async def demo_health():
    return await _fwd("get", "/health")

@app.get("/demo/metrics")
async def demo_metrics():
    return await _fwd("get", "/metrics")


# ---------------------------------------------------------------------------
# Section 3: Full Feature Tour
# ---------------------------------------------------------------------------

@app.post("/demo/tour")
async def demo_tour():
    log: list[dict] = []

    async def step(name: str, method: str, path: str, **kwargs) -> dict:
        result = await _fwd(method, path, **kwargs)
        entry = {"step": name, **result}
        log.append(entry)
        print(f"  tour: {name} -> {result['status']}")
        return result

    print("\n=== mininq Feature Tour ===\n")

    # 1. Health check
    await step("health_check", "get", "/health")

    # 2. Create demo-queue
    await step("create_queue", "post", "/queues", json={
        "name": "demo-queue",
        "max_concurrency": 2,
        "rate_limit_rps": 5.0,
        "retry_backoff": "linear",
        "base_delay_ms": 1000,
        "max_delay_ms": 10000,
    })

    # 3. Basic job
    await step("basic_job", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"msg": "hello from tour"},
    })

    # 4. Priority jobs (1, 5, 10 — highest runs first)
    for p in [1, 5, 10]:
        await step(f"priority_job_p{p}", "post", "/jobs", json={
            "queue_name": "demo-queue",
            "callback_url": f"{SELF}/webhook/generic",
            "payload": {"priority_test": True, "priority": p},
            "priority": p,
        })

    # 5. Delayed job
    await step("delayed_job", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"delayed": True},
        "delay_ms": 5000,
    })

    # 6. Idempotent job — same key twice
    idem_body = {
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"idempotent": True},
        "idempotency_key": "tour-idem-001",
    }
    first = await step("idempotent_first", "post", "/jobs", json=idem_body)
    second = await step("idempotent_duplicate", "post", "/jobs", json=idem_body)
    log.append({
        "step": "idempotent_check",
        "first_status": first["status"],
        "second_status": second["status"],
        "deduped": first["status"] == 201 and second["status"] == 200,
    })

    # 7. Flaky job (retries with backoff)
    await step("flaky_job", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/flaky",
        "payload": {"flaky": True},
        "max_retries": 3,
        "retry_backoff": "fixed",
        "base_delay_ms": 500,
    })

    # 8. Failing jobs
    await step("always_fail_job", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/always-fail",
        "payload": {"will_die": True},
        "max_retries": 1,
        "retry_backoff": "fixed",
        "base_delay_ms": 500,
    })
    await step("permanent_fail_job", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/permanent-fail",
        "payload": {"permanent_fail": True},
    })

    # 9. Pause/resume
    await step("pause_queue", "post", "/queues/demo-queue/pause")
    await step("job_while_paused", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"enqueued_while_paused": True},
    })
    await asyncio.sleep(1)
    await step("queue_stats_while_paused", "get", "/queues")
    await step("resume_queue", "post", "/queues/demo-queue/resume")

    # 10. Cron schedule
    sched = await step("create_schedule", "post", "/schedules", json={
        "queue_name": "demo-queue",
        "cron_expression": "0 * * * * *",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"scheduled": True},
    })
    await step("list_schedules", "get", "/schedules")
    schedule_id = sched["body"].get("id", "") if isinstance(sched["body"], dict) else ""
    if schedule_id:
        await step("disable_schedule", "put", f"/schedules/{schedule_id}", json={"enabled": False})

    # 11. Wait for jobs to process, then fetch metrics
    print("  tour: waiting 3s for jobs to process...")
    await asyncio.sleep(3)
    await step("metrics", "get", "/metrics")
    await step("queue_stats", "get", "/queues")

    # 12. List dead jobs and retry one
    dead_jobs = await step("list_dead_jobs", "get", "/jobs?queue=demo-queue&status=dead&limit=5")
    if isinstance(dead_jobs["body"], list) and len(dead_jobs["body"]) > 0:
        dead_id = dead_jobs["body"][0]["id"]
        await step("retry_dead_job", "post", f"/jobs/{dead_id}/retry")

    # 13. Create a long-delayed job and cancel it
    cancel_target = await step("create_cancel_target", "post", "/jobs", json={
        "queue_name": "demo-queue",
        "callback_url": f"{SELF}/webhook/generic",
        "payload": {"to_be_cancelled": True},
        "delay_ms": 600000,
    })
    cancel_id = ""
    if isinstance(cancel_target["body"], dict):
        job_data = cancel_target["body"].get("job", cancel_target["body"])
        cancel_id = job_data.get("id", "")
    if cancel_id:
        await step("cancel_job", "delete", f"/jobs/{cancel_id}")

    # 14. Cleanup
    if schedule_id:
        await step("delete_schedule", "delete", f"/schedules/{schedule_id}")

    print("\n=== Tour Complete ===\n")
    return {"tour": "complete", "steps": len(log), "log": log}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8000)
