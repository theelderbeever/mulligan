use crate::backoff::Backoff;
use crate::jitter::Jitter;
use crate::retry_policy::RetryPolicy;

/// A blocking iterator over attempts produced by a retry policy.
pub struct Attempts<Back: Backoff, Jit: Jitter> {
    current: u32,
    policy: RetryPolicy<Back, Jit>,
}

impl<Back: Backoff, Jit: Jitter> Attempts<Back, Jit> {
    pub(crate) fn new(policy: RetryPolicy<Back, Jit>) -> Self {
        Self { current: 0, policy }
    }
}

impl<Back: Backoff, Jit: Jitter> Iterator for Attempts<Back, Jit> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.policy.should_stop(self.current) {
            return None;
        }

        if self.current > 0 {
            let delay = self.policy.calculate_delay(self.current - 1);
            std::thread::sleep(delay);
        }

        let attempt = self.current;
        self.current += 1;
        Some(attempt)
    }
}

#[cfg(test)]
mod tests {
    use crate::retry;

    #[test]
    fn stop_after_limits_retries_after_the_initial_attempt() {
        let attempts = retry().stop_after(3).attempts_sync().collect::<Vec<_>>();

        assert_eq!(attempts, vec![0, 1, 2, 3]);
    }

    #[test]
    fn zero_retries_still_yields_the_initial_attempt() {
        assert_eq!(
            retry().stop_after(0).attempts_sync().collect::<Vec<_>>(),
            vec![0]
        );
    }
}
