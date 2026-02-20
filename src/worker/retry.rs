use rand::Rng;

/// Computes the next visible_at delay in milliseconds based on retry strategy.
pub fn compute_delay_ms(
    attempt: i32,
    backoff: &str,
    base_delay_ms: i32,
    max_delay_ms: i32,
) -> i64 {
    let raw = match backoff {
        "linear" => base_delay_ms as i64 * attempt as i64,
        "fixed" => base_delay_ms as i64,
        _ => {
            // exponential: base * 2^(attempt-1)
            let exp = (attempt - 1).max(0) as u32;
            (base_delay_ms as i64).saturating_mul(2_i64.saturating_pow(exp))
        }
    };

    // Add 30% jitter
    let mut rng = rand::thread_rng();
    let jitter_factor = 1.0 + rng.gen_range(-0.3..0.3);
    let with_jitter = (raw as f64 * jitter_factor) as i64;

    with_jitter.min(max_delay_ms as i64).max(0)
}
