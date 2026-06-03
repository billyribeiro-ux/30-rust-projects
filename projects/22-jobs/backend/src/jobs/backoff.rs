//! Exponential backoff with jitter.
//!
//! Schedule: base × factor^(attempt-1), capped at `max_secs`, with up
//! to ±25% jitter (decorrelated). Attempts start at 1 = "the run that
//! just failed".
//!
//! The math is in one place so it can be proptested in isolation.

use rand::RngExt;

#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    pub base_secs: f64,
    pub factor: f64,
    pub max_secs: f64,
}

impl Default for Backoff {
    fn default() -> Self {
        // 30s, 90s, 4.5min, 13.5min, … capped at 1h.
        Self {
            base_secs: 30.0,
            factor: 3.0,
            max_secs: 3600.0,
        }
    }
}

impl Backoff {
    pub fn delay_secs(&self, attempt: u32) -> f64 {
        let base = self.base_secs * self.factor.powi((attempt.max(1) - 1) as i32);
        let capped = base.min(self.max_secs);
        let mut rng = rand::rng();
        let jitter_factor = 1.0 + rng.random_range(-0.25..=0.25);
        (capped * jitter_factor).max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_monotonic_non_decreasing_in_expectation() {
        let b = Backoff::default();
        // 1000 samples per attempt → median should be roughly base*factor^(n-1).
        for attempts in [1, 2, 3, 4, 5] {
            let mut samples: Vec<f64> = (0..1000).map(|_| b.delay_secs(attempts)).collect();
            samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = samples[500];
            let expected_floor = b.base_secs * b.factor.powi((attempts as i32) - 1) * 0.74;
            assert!(
                median >= expected_floor,
                "attempt {attempts}: median {median} below expected floor {expected_floor}"
            );
        }
    }

    #[test]
    fn cap_holds() {
        let b = Backoff {
            base_secs: 1.0,
            factor: 10.0,
            max_secs: 50.0,
        };
        // attempt=10 would be 1 * 10^9 without cap.
        let delays: Vec<f64> = (0..200).map(|_| b.delay_secs(10)).collect();
        let max = delays.iter().copied().fold(f64::MIN, f64::max);
        assert!(max <= 50.0 * 1.26, "cap × jitter ceiling exceeded: {max}");
    }
}
