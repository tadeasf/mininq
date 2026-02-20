use std::collections::HashMap;
use std::time::Instant;

struct TokenBucket {
    rate: f64,
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(rate: f64) -> Self {
        Self {
            rate,
            capacity: rate.max(1.0),
            tokens: rate.max(1.0),
            last_refill: Instant::now(),
        }
    }

    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate).min(self.capacity);
        self.last_refill = now;
    }

    fn update_rate(&mut self, rate: f64) {
        if (self.rate - rate).abs() > f64::EPSILON {
            self.rate = rate;
            self.capacity = rate.max(1.0);
        }
    }
}

pub struct RateLimiter {
    buckets: HashMap<String, TokenBucket>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Returns true if the queue is allowed to process a job.
    /// Queues with no rate limit always pass.
    pub fn check_queue(&mut self, name: &str, rate_limit_rps: Option<f64>) -> bool {
        let rate = match rate_limit_rps {
            Some(r) if r > 0.0 => r,
            _ => return true,
        };

        let bucket = self.buckets.entry(name.to_string()).or_insert_with(|| {
            TokenBucket::new(rate)
        });
        bucket.update_rate(rate);
        bucket.try_acquire()
    }
}
