#!/usr/bin/env bash
#
# mininq curl examples
#
# Prerequisites:
#   1. cargo run                          (mininq on :6390)
#   2. python examples/main.py            (example backend on :8000)
#      OR  bun run examples/server.ts     (example backend on :3000)
#
# Usage:
#   Run the whole file:  bash examples/docs/curl-examples.sh
#   Or copy individual commands into your terminal.
#
# Adjust BACKEND if using the Bun server on port 3000.

set -euo pipefail

MININQ="http://localhost:6390"
BACKEND="http://localhost:8008"  # change to :3000 for Bun

pp() { python3 -m json.tool 2>/dev/null || cat; }
hr() { echo -e "\n--- $1 ---\n"; }

# ============================================================================
# 1. HEALTH & METRICS (direct mininq calls)
# ============================================================================

hr "Health check"
curl -s "$MININQ/health" | pp

hr "Metrics"
curl -s "$MININQ/metrics" | pp

# ============================================================================
# 2. QUEUE MANAGEMENT
# ============================================================================

hr "Create queue"
curl -s -X POST "$MININQ/queues" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "demo-queue",
    "max_concurrency": 2,
    "rate_limit_rps": 5.0,
    "retry_backoff": "linear",
    "base_delay_ms": 1000,
    "max_delay_ms": 10000
  }' | pp

hr "List queues"
curl -s "$MININQ/queues" | pp

hr "Update queue (bump concurrency to 4)"
curl -s -X PUT "$MININQ/queues/demo-queue" \
  -H "Content-Type: application/json" \
  -d '{"max_concurrency": 4}' | pp

hr "Pause queue"
curl -s -X POST "$MININQ/queues/demo-queue/pause" | pp

hr "Resume queue"
curl -s -X POST "$MININQ/queues/demo-queue/resume" | pp

# ============================================================================
# 3. BASIC JOB
# ============================================================================

hr "Enqueue a basic job"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"msg\": \"hello from curl\"}
  }" | pp

# ============================================================================
# 4. PRIORITY JOBS
# ============================================================================

hr "Enqueue priority=1 (low)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"priority\": 1},
    \"priority\": 1
  }" | pp

hr "Enqueue priority=5 (medium)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"priority\": 5},
    \"priority\": 5
  }" | pp

hr "Enqueue priority=10 (high — runs first)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"priority\": 10},
    \"priority\": 10
  }" | pp

# ============================================================================
# 5. DELAYED JOB
# ============================================================================

hr "Enqueue delayed job (5s)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"delayed\": true},
    \"delay_ms\": 5000
  }" | pp

# ============================================================================
# 6. IDEMPOTENT JOB
# ============================================================================

hr "Idempotent job — first call (expect 201)"
curl -s -w "\nHTTP %{http_code}\n" -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"idempotent\": true},
    \"idempotency_key\": \"curl-idem-001\"
  }" | pp

hr "Idempotent job — duplicate call (expect 200, same job returned)"
curl -s -w "\nHTTP %{http_code}\n" -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"idempotent\": true},
    \"idempotency_key\": \"curl-idem-001\"
  }" | pp

# ============================================================================
# 7. RETRY STRATEGIES
# ============================================================================

hr "Flaky job — fixed backoff, 3 retries (succeeds on retry)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/flaky\",
    \"payload\": {\"flaky\": true},
    \"max_retries\": 3,
    \"retry_backoff\": \"fixed\",
    \"base_delay_ms\": 500
  }" | pp

hr "Exponential backoff job"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/flaky\",
    \"payload\": {\"exponential\": true},
    \"max_retries\": 3,
    \"retry_backoff\": \"exponential\",
    \"base_delay_ms\": 500,
    \"max_delay_ms\": 10000
  }" | pp

# ============================================================================
# 8. DEAD-LETTERING
# ============================================================================

hr "Always-fail job (5xx -> retries exhausted -> dead)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/always-fail\",
    \"payload\": {\"will_die\": true},
    \"max_retries\": 1,
    \"retry_backoff\": \"fixed\",
    \"base_delay_ms\": 500
  }" | pp

hr "Permanent-fail job (4xx -> immediate dead, no retry)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/permanent-fail\",
    \"payload\": {\"permanent_fail\": true}
  }" | pp

# ============================================================================
# 9. SLOW JOB (timeout testing)
# ============================================================================

hr "Slow job (2s callback, custom timeout)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/slow\",
    \"payload\": {\"slow\": true},
    \"timeout_ms\": 10000
  }" | pp

# ============================================================================
# 10. JOB INSPECTION
# ============================================================================

hr "List jobs in demo-queue"
curl -s "$MININQ/jobs?queue=demo-queue&limit=10" | pp

hr "List only dead jobs"
curl -s "$MININQ/jobs?queue=demo-queue&status=dead&limit=10" | pp

hr "List only pending jobs"
curl -s "$MININQ/jobs?queue=demo-queue&status=pending&limit=10" | pp

# To get/cancel/retry a specific job, grab an ID from the list above:
#
#   hr "Get a specific job"
#   curl -s "$MININQ/jobs/<JOB_ID>" | pp
#
#   hr "Cancel a pending job"
#   curl -s -X DELETE "$MININQ/jobs/<JOB_ID>" | pp
#
#   hr "Retry a dead job"
#   curl -s -X POST "$MININQ/jobs/<JOB_ID>/retry" | pp

# ============================================================================
# 11. PAUSE / RESUME WORKFLOW
# ============================================================================

hr "Pause demo-queue"
curl -s -X POST "$MININQ/queues/demo-queue/pause" | pp

hr "Enqueue job while paused (stays pending)"
curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"enqueued_while_paused\": true}
  }" | pp

hr "Check queue stats (should show paused=true, pending > 0)"
curl -s "$MININQ/queues" | pp

sleep 1

hr "Resume demo-queue (paused job starts processing)"
curl -s -X POST "$MININQ/queues/demo-queue/resume" | pp

# ============================================================================
# 12. CRON SCHEDULES
# ============================================================================

hr "Create schedule (every minute)"
SCHEDULE_RESPONSE=$(curl -s -X POST "$MININQ/schedules" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"cron_expression\": \"0 * * * * *\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"scheduled\": true}
  }")
echo "$SCHEDULE_RESPONSE" | pp
SCHEDULE_ID=$(echo "$SCHEDULE_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")

hr "List schedules"
curl -s "$MININQ/schedules" | pp

if [ -n "$SCHEDULE_ID" ]; then
  hr "Get schedule by ID"
  curl -s "$MININQ/schedules/$SCHEDULE_ID" | pp

  hr "Disable schedule"
  curl -s -X PUT "$MININQ/schedules/$SCHEDULE_ID" \
    -H "Content-Type: application/json" \
    -d '{"enabled": false}' | pp

  hr "Update schedule cron to every 5 minutes"
  curl -s -X PUT "$MININQ/schedules/$SCHEDULE_ID" \
    -H "Content-Type: application/json" \
    -d '{"cron_expression": "0 */5 * * * *", "enabled": true}' | pp

  hr "Delete schedule"
  curl -s -X DELETE "$MININQ/schedules/$SCHEDULE_ID" | pp
fi

# ============================================================================
# 13. JOB CANCELLATION
# ============================================================================

hr "Create a long-delayed job (10 min) then cancel it"
CANCEL_RESPONSE=$(curl -s -X POST "$MININQ/jobs" \
  -H "Content-Type: application/json" \
  -d "{
    \"queue_name\": \"demo-queue\",
    \"callback_url\": \"$BACKEND/webhook/generic\",
    \"payload\": {\"to_be_cancelled\": true},
    \"delay_ms\": 600000
  }")
echo "$CANCEL_RESPONSE" | pp
CANCEL_ID=$(echo "$CANCEL_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['job']['id'])" 2>/dev/null || echo "")

if [ -n "$CANCEL_ID" ]; then
  hr "Cancel the delayed job"
  curl -s -X DELETE "$MININQ/jobs/$CANCEL_ID" | pp
fi

# ============================================================================
# 14. DEAD JOB RETRY
# ============================================================================

hr "Wait 3s for failing jobs to exhaust retries..."
sleep 3

hr "List dead jobs"
DEAD_RESPONSE=$(curl -s "$MININQ/jobs?queue=demo-queue&status=dead&limit=1")
echo "$DEAD_RESPONSE" | pp
DEAD_ID=$(echo "$DEAD_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['id'] if d else '')" 2>/dev/null || echo "")

if [ -n "$DEAD_ID" ]; then
  hr "Retry dead job $DEAD_ID"
  curl -s -X POST "$MININQ/jobs/$DEAD_ID/retry" | pp
fi

# ============================================================================
# 15. FINAL METRICS
# ============================================================================

hr "Final metrics snapshot"
curl -s "$MININQ/metrics" | pp

hr "Final queue stats"
curl -s "$MININQ/queues" | pp

# ============================================================================
# 16. CLEANUP (optional)
# ============================================================================

# hr "Delete demo-queue (only works when no active jobs remain)"
# curl -s -X DELETE "$MININQ/queues/demo-queue" | pp

# ============================================================================
# 17. FULL TOUR (via example backend)
# ============================================================================

# Runs every feature in sequence and returns a JSON log:
#
#   curl -s -X POST "$BACKEND/demo/tour" | pp

echo -e "\n=== All curl examples complete ===\n"
