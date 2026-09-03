use std::time::Duration;

use crate::backoff::{Backoff, Exponential, Fixed, Linear};
use crate::jitter::{Jitter, NoJitter};
use crate::retry_policy::retry_policy_builder;

/// Create a sync retry iterator with default settings (no delay, no limit).
pub fn iter() -> RetryIter<Fixed, NoJitter> {
    RetryIter::new()
}

pub struct RetryIter<Back: Backoff, Jit: Jitter> {
    current: u32,
    stop_after: Option<u32>,
    backoff: Back,
    jitterable: Jit,
    max: Option<Duration>,
}

retry_policy_builder!(RetryIter {});

impl<Back: Backoff, Jit: Jitter> Iterator for RetryIter<Back, Jit> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop_after.is_some_and(|max| self.current > max) {
            return None;
        }

        if self.current == 0 {
            self.current += 1;
            return Some(0);
        }

        let delay = self.calculate_delay(self.current - 1);
        std::thread::sleep(delay);

        let attempt = self.current;
        self.current += 1;
        Some(attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::iter;

    #[test]
    fn stop_after_limits_retries_after_the_initial_attempt() {
        assert_eq!(iter().stop_after(3).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn zero_retries_still_yields_the_initial_attempt() {
        assert_eq!(iter().stop_after(0).collect::<Vec<_>>(), vec![0]);
    }
}
