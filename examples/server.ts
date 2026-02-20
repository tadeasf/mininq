/**
 * mininq example backend — Hono on Bun
 *
 * Callback receivers (webhook targets) + demo client that exercises every mininq API endpoint.
 *
 * Dependencies: bun add hono
 * Run:          bun run examples/server.ts
 *               (mininq must be running on port 6390)
 */

import { Hono } from "hono";

const MININQ = "http://localhost:6390";
const SELF = "http://localhost:3000";

const app = new Hono();

// Track first-attempt per job for the flaky endpoint
const seenJobs = new Set<string>();

// ---------------------------------------------------------------------------
// Section 1: Callback Receivers
// ---------------------------------------------------------------------------

app.post("/webhook/generic", async (c) => {
  const body = await c.req.json();
  const jobId = c.req.header("x-mininq-job-id") ?? "?";
  const attempt = c.req.header("x-mininq-attempt") ?? "?";
  const queue = c.req.header("x-mininq-queue") ?? "?";
  console.log(
    `[generic]  job=${jobId} attempt=${attempt} queue=${queue} payload=${JSON.stringify(body)}`,
  );
  return c.json({ received: true });
});

app.post("/webhook/slow", async (c) => {
  const jobId = c.req.header("x-mininq-job-id") ?? "?";
  console.log(`[slow]     job=${jobId} sleeping 2s...`);
  await Bun.sleep(2000);
  console.log(`[slow]     job=${jobId} done`);
  return c.json({ received: true, delayed: true });
});

app.post("/webhook/flaky", async (c) => {
  const jobId = c.req.header("x-mininq-job-id") ?? "unknown";
  const attempt = c.req.header("x-mininq-attempt") ?? "?";
  if (!seenJobs.has(jobId)) {
    seenJobs.add(jobId);
    console.log(`[flaky]    job=${jobId} attempt=${attempt} -> 500 (first try)`);
    return c.json({ error: "flaky failure" }, 500);
  }
  console.log(`[flaky]    job=${jobId} attempt=${attempt} -> 200 (retry OK)`);
  return c.json({ received: true, retried: true });
});

app.post("/webhook/always-fail", async (c) => {
  const jobId = c.req.header("x-mininq-job-id") ?? "?";
  const attempt = c.req.header("x-mininq-attempt") ?? "?";
  console.log(`[fail]     job=${jobId} attempt=${attempt} -> 500`);
  return c.json({ error: "always fails" }, 500);
});

app.post("/webhook/permanent-fail", async (c) => {
  const jobId = c.req.header("x-mininq-job-id") ?? "?";
  console.log(`[perm-fail] job=${jobId} -> 400 (immediate dead)`);
  return c.json({ error: "bad request, no retry" }, 400);
});

// ---------------------------------------------------------------------------
// Section 2: Demo Endpoints (individual feature demos)
// ---------------------------------------------------------------------------

type Method = "GET" | "POST" | "PUT" | "DELETE";

interface StepResult {
  step?: string;
  status: number;
  body: unknown;
}

async function fwd(
  method: Method,
  path: string,
  body?: unknown,
): Promise<StepResult> {
  const opts: RequestInit = {
    method,
    headers: { "Content-Type": "application/json" },
  };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const r = await fetch(`${MININQ}${path}`, opts);
  let data: unknown;
  try {
    data = await r.json();
  } catch {
    data = await r.text();
  }
  return { status: r.status, body: data };
}

// Queue management

app.post("/demo/create-queue", async (c) => {
  return c.json(
    await fwd("POST", "/queues", {
      name: "demo-queue",
      max_concurrency: 2,
      rate_limit_rps: 5.0,
      retry_backoff: "linear",
      base_delay_ms: 1000,
      max_delay_ms: 10000,
    }),
  );
});

app.get("/demo/list-queues", async (c) => {
  return c.json(await fwd("GET", "/queues"));
});

app.put("/demo/update-queue", async (c) => {
  return c.json(
    await fwd("PUT", "/queues/demo-queue", { max_concurrency: 4 }),
  );
});

app.post("/demo/pause-queue", async (c) => {
  return c.json(await fwd("POST", "/queues/demo-queue/pause"));
});

app.post("/demo/resume-queue", async (c) => {
  return c.json(await fwd("POST", "/queues/demo-queue/resume"));
});

app.delete("/demo/delete-queue", async (c) => {
  return c.json(await fwd("DELETE", "/queues/demo-queue"));
});

// Job lifecycle

app.post("/demo/basic-job", async (c) => {
  return c.json(
    await fwd("POST", "/jobs", {
      queue_name: "demo-queue",
      callback_url: `${SELF}/webhook/generic`,
      payload: { msg: "hello from demo" },
    }),
  );
});

app.post("/demo/priority-jobs", async (c) => {
  const results: StepResult[] = [];
  for (const p of [1, 5, 10]) {
    results.push(
      await fwd("POST", "/jobs", {
        queue_name: "demo-queue",
        callback_url: `${SELF}/webhook/generic`,
        payload: { priority_test: true, priority: p },
        priority: p,
      }),
    );
  }
  return c.json(results);
});

app.post("/demo/delayed-job", async (c) => {
  return c.json(
    await fwd("POST", "/jobs", {
      queue_name: "demo-queue",
      callback_url: `${SELF}/webhook/generic`,
      payload: { delayed: true },
      delay_ms: 5000,
    }),
  );
});

app.post("/demo/idempotent-job", async (c) => {
  const body = {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { idempotent: true },
    idempotency_key: "demo-idem-key-001",
  };
  const first = await fwd("POST", "/jobs", body);
  const second = await fwd("POST", "/jobs", body);
  return c.json({ first, second });
});

app.post("/demo/retry-job", async (c) => {
  return c.json(
    await fwd("POST", "/jobs", {
      queue_name: "demo-queue",
      callback_url: `${SELF}/webhook/flaky`,
      payload: { flaky: true },
      max_retries: 3,
      retry_backoff: "fixed",
      base_delay_ms: 500,
    }),
  );
});

app.post("/demo/failing-job", async (c) => {
  const alwaysFail = await fwd("POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/always-fail`,
    payload: { will_die: true },
    max_retries: 1,
    retry_backoff: "fixed",
    base_delay_ms: 500,
  });
  const permanentFail = await fwd("POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/permanent-fail`,
    payload: { permanent_fail: true },
  });
  return c.json({ always_fail: alwaysFail, permanent_fail: permanentFail });
});

app.get("/demo/list-jobs", async (c) => {
  return c.json(await fwd("GET", "/jobs?queue=demo-queue&limit=20"));
});

app.get("/demo/get-job/:jobId", async (c) => {
  const jobId = c.req.param("jobId");
  return c.json(await fwd("GET", `/jobs/${jobId}`));
});

app.delete("/demo/cancel-job/:jobId", async (c) => {
  const jobId = c.req.param("jobId");
  return c.json(await fwd("DELETE", `/jobs/${jobId}`));
});

app.post("/demo/retry-dead-job/:jobId", async (c) => {
  const jobId = c.req.param("jobId");
  return c.json(await fwd("POST", `/jobs/${jobId}/retry`));
});

// Schedules

app.post("/demo/create-schedule", async (c) => {
  return c.json(
    await fwd("POST", "/schedules", {
      queue_name: "demo-queue",
      cron_expression: "0 * * * * *",
      callback_url: `${SELF}/webhook/generic`,
      payload: { scheduled: true },
    }),
  );
});

app.get("/demo/list-schedules", async (c) => {
  return c.json(await fwd("GET", "/schedules"));
});

app.put("/demo/update-schedule/:scheduleId", async (c) => {
  const scheduleId = c.req.param("scheduleId");
  return c.json(
    await fwd("PUT", `/schedules/${scheduleId}`, { enabled: false }),
  );
});

app.delete("/demo/delete-schedule/:scheduleId", async (c) => {
  const scheduleId = c.req.param("scheduleId");
  return c.json(await fwd("DELETE", `/schedules/${scheduleId}`));
});

// Observability

app.get("/demo/health", async (c) => {
  return c.json(await fwd("GET", "/health"));
});

app.get("/demo/metrics", async (c) => {
  return c.json(await fwd("GET", "/metrics"));
});

// ---------------------------------------------------------------------------
// Section 3: Full Feature Tour
// ---------------------------------------------------------------------------

app.post("/demo/tour", async (c) => {
  const log: Record<string, unknown>[] = [];

  async function step(
    name: string,
    method: Method,
    path: string,
    body?: unknown,
  ): Promise<StepResult> {
    const result = await fwd(method, path, body);
    const entry = { step: name, ...result };
    log.push(entry);
    console.log(`  tour: ${name} -> ${result.status}`);
    return result;
  }

  console.log("\n=== mininq Feature Tour ===\n");

  // 1. Health check
  await step("health_check", "GET", "/health");

  // 2. Create demo-queue
  await step("create_queue", "POST", "/queues", {
    name: "demo-queue",
    max_concurrency: 2,
    rate_limit_rps: 5.0,
    retry_backoff: "linear",
    base_delay_ms: 1000,
    max_delay_ms: 10000,
  });

  // 3. Basic job
  await step("basic_job", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { msg: "hello from tour" },
  });

  // 4. Priority jobs (1, 5, 10 — highest runs first)
  for (const p of [1, 5, 10]) {
    await step(`priority_job_p${p}`, "POST", "/jobs", {
      queue_name: "demo-queue",
      callback_url: `${SELF}/webhook/generic`,
      payload: { priority_test: true, priority: p },
      priority: p,
    });
  }

  // 5. Delayed job
  await step("delayed_job", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { delayed: true },
    delay_ms: 5000,
  });

  // 6. Idempotent job — same key twice
  const idemBody = {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { idempotent: true },
    idempotency_key: "tour-idem-001",
  };
  const first = await step("idempotent_first", "POST", "/jobs", idemBody);
  const second = await step(
    "idempotent_duplicate",
    "POST",
    "/jobs",
    idemBody,
  );
  log.push({
    step: "idempotent_check",
    first_status: first.status,
    second_status: second.status,
    deduped: first.status === 201 && second.status === 200,
  });

  // 7. Flaky job (retries with backoff)
  await step("flaky_job", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/flaky`,
    payload: { flaky: true },
    max_retries: 3,
    retry_backoff: "fixed",
    base_delay_ms: 500,
  });

  // 8. Failing jobs
  await step("always_fail_job", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/always-fail`,
    payload: { will_die: true },
    max_retries: 1,
    retry_backoff: "fixed",
    base_delay_ms: 500,
  });
  await step("permanent_fail_job", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/permanent-fail`,
    payload: { permanent_fail: true },
  });

  // 9. Pause/resume
  await step("pause_queue", "POST", "/queues/demo-queue/pause");
  await step("job_while_paused", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { enqueued_while_paused: true },
  });
  await Bun.sleep(1000);
  await step("queue_stats_while_paused", "GET", "/queues");
  await step("resume_queue", "POST", "/queues/demo-queue/resume");

  // 10. Cron schedule
  const sched = await step("create_schedule", "POST", "/schedules", {
    queue_name: "demo-queue",
    cron_expression: "0 * * * * *",
    callback_url: `${SELF}/webhook/generic`,
    payload: { scheduled: true },
  });
  await step("list_schedules", "GET", "/schedules");
  const scheduleId =
    typeof sched.body === "object" && sched.body !== null
      ? (sched.body as Record<string, unknown>).id
      : "";
  if (scheduleId) {
    await step("disable_schedule", "PUT", `/schedules/${scheduleId}`, {
      enabled: false,
    });
  }

  // 11. Wait for jobs to process, then fetch metrics
  console.log("  tour: waiting 3s for jobs to process...");
  await Bun.sleep(3000);
  await step("metrics", "GET", "/metrics");
  await step("queue_stats", "GET", "/queues");

  // 12. List dead jobs and retry one
  const deadJobs = await step(
    "list_dead_jobs",
    "GET",
    "/jobs?queue=demo-queue&status=dead&limit=5",
  );
  if (Array.isArray(deadJobs.body) && deadJobs.body.length > 0) {
    const deadId = (deadJobs.body[0] as Record<string, unknown>).id;
    await step("retry_dead_job", "POST", `/jobs/${deadId}/retry`);
  }

  // 13. Create a long-delayed job and cancel it
  const cancelTarget = await step("create_cancel_target", "POST", "/jobs", {
    queue_name: "demo-queue",
    callback_url: `${SELF}/webhook/generic`,
    payload: { to_be_cancelled: true },
    delay_ms: 600000,
  });
  let cancelId = "";
  if (typeof cancelTarget.body === "object" && cancelTarget.body !== null) {
    const jobData =
      (cancelTarget.body as Record<string, unknown>).job ??
      cancelTarget.body;
    cancelId = String(
      (jobData as Record<string, unknown>).id ?? "",
    );
  }
  if (cancelId) {
    await step("cancel_job", "DELETE", `/jobs/${cancelId}`);
  }

  // 14. Cleanup
  if (scheduleId) {
    await step("delete_schedule", "DELETE", `/schedules/${scheduleId}`);
  }

  console.log("\n=== Tour Complete ===\n");
  return c.json({ tour: "complete", steps: log.length, log });
});

// ---------------------------------------------------------------------------
// Start server
// ---------------------------------------------------------------------------

console.log(`\n  Example server ready on ${SELF}`);
console.log(`  mininq expected at      ${MININQ}`);
console.log(`  POST ${SELF}/demo/tour to run the full feature tour\n`);

export default {
  port: 3000,
  fetch: app.fetch,
};
