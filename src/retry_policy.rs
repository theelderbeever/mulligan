use std::time::Duration;

use crate::backoff::{Backoff, Exponential, Fixed, Linear};
use crate::blocking::Attempts;
#[cfg(feature = "iter")]
use crate::iter::AsyncAttempts;
use crate::jitter::{Decorrelated, Equal, Full, Jitter, NoJitter};

/// Creates a retry policy with no delay and no retry limit.
pub fn retry() -> RetryPolicy<Fixed, NoJitter> {
    RetryPolicy::new()
}

/// Configuration shared by asynchronous and synchronous attempt iterators.
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct RetryPolicy<Back: Backoff, Jit: Jitter> {
    pub(crate) stop_after: Option<u32>,
    pub(crate) backoff: Back,
    pub(crate) jitter: Jit,
    #[cfg_attr(
        feature = "serde",
        serde(default, deserialize_with = "deserialize_optional_duration")
    )]
    pub(crate) max_delay: Option<Duration>,
}

#[cfg(feature = "serde")]
fn deserialize_optional_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let duration =
        <Option<duration_string::DurationString> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(duration.map(Into::into))
}

impl Default for RetryPolicy<Fixed, NoJitter> {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryPolicy<Fixed, NoJitter> {
    pub fn new() -> Self {
        Self {
            stop_after: None,
            backoff: Fixed::base(Duration::ZERO),
            jitter: NoJitter,
            max_delay: None,
        }
    }
}

impl<Back: Backoff, Jit: Jitter> RetryPolicy<Back, Jit> {
    /// Produces asynchronous attempts using this policy.
    #[cfg(feature = "iter")]
    pub fn attempts(self) -> AsyncAttempts<Back, Jit> {
        AsyncAttempts::new(self)
    }

    /// Produces blocking attempts using this policy.
    pub fn attempts_sync(self) -> Attempts<Back, Jit> {
        Attempts::new(self)
    }

    /// Sets the maximum number of retries after the initial attempt.
    pub fn stop_after(mut self, retries: u32) -> Self {
        self.stop_after = Some(retries);
        self
    }

    /// Caps the maximum delay between retries.
    pub fn max_delay(mut self, dur: Duration) -> Self {
        self.max_delay = Some(dur);
        self
    }

    /// Waits a fixed amount of time between each retry.
    pub fn fixed(self, dur: Duration) -> RetryPolicy<Fixed, Jit> {
        RetryPolicy {
            stop_after: self.stop_after,
            backoff: Fixed::base(dur),
            jitter: self.jitter,
            max_delay: self.max_delay,
        }
    }

    /// Waits a linearly growing amount of time between each retry `base * attempt`.
    pub fn linear(self, dur: Duration) -> RetryPolicy<Linear, Jit> {
        RetryPolicy {
            stop_after: self.stop_after,
            backoff: Linear::base(dur),
            jitter: self.jitter,
            max_delay: self.max_delay,
        }
    }

    /// Waits an exponentially growing amount of time between each retry `base * 2^attempt`.
    /// Use [`RetryPolicy::backoff`] with [`Exponential::multiplier`] to customize the multiplier.
    pub fn exponential(self, dur: Duration) -> RetryPolicy<Exponential, Jit> {
        RetryPolicy {
            stop_after: self.stop_after,
            backoff: Exponential::base(dur),
            jitter: self.jitter,
            max_delay: self.max_delay,
        }
    }

    /// Uses a custom backoff strategy.
    pub fn backoff<B: Backoff>(self, backoff: B) -> RetryPolicy<B, Jit> {
        RetryPolicy {
            stop_after: self.stop_after,
            backoff,
            jitter: self.jitter,
            max_delay: self.max_delay,
        }
    }

    /// Uses a custom jitter strategy.
    pub fn jitter<J: Jitter>(self, jitter: J) -> RetryPolicy<Back, J> {
        RetryPolicy {
            stop_after: self.stop_after,
            backoff: self.backoff,
            jitter,
            max_delay: self.max_delay,
        }
    }

    /// Adds a random delay between zero and the backoff value.
    pub fn full_jitter(self) -> RetryPolicy<Back, Full> {
        self.jitter(Full)
    }

    /// Adds a random delay between half the backoff and the full backoff value.
    pub fn equal_jitter(self) -> RetryPolicy<Back, Equal> {
        self.jitter(Equal)
    }

    /// Adds decorrelated jitter based on the previous delay.
    pub fn decorrelated_jitter(self, base: Duration) -> RetryPolicy<Back, Decorrelated> {
        self.jitter(Decorrelated::base(base))
    }

    pub(crate) fn should_stop(&self, current: u32) -> bool {
        self.stop_after.is_some_and(|max| current > max)
    }

    pub(crate) fn calculate_delay(&mut self, attempt: u32) -> Duration {
        let delay = self.backoff.delay(attempt);
        self.jitter.jitter(delay, self.max_delay)
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use std::time::Duration;

    use crate::{Backoff, Exponential, Full, NoJitter};

    use super::RetryPolicy;

    #[test]
    fn deserializes_a_retry_policy_with_duration_strings() {
        let policy: RetryPolicy<Exponential, Full> = serde_json::from_str(
            r#"{
                "stop_after": 4,
                "backoff": { "kind": "exponential", "base": "250ms" },
                "jitter": { "kind": "full" },
                "max_delay": "3s"
            }"#,
        )
        .unwrap();

        assert_eq!(policy.stop_after, Some(4));
        assert_eq!(policy.backoff.base(), Duration::from_millis(250));
        assert_eq!(policy.max_delay, Some(Duration::from_secs(3)));
    }

    #[test]
    fn defaults_to_no_retry_limit_or_maximum_delay() {
        let policy: RetryPolicy<Exponential, NoJitter> = serde_json::from_str(
            r#"{
                "backoff": { "kind": "exponential", "base": "1s" },
                "jitter": { "kind": "none" }
            }"#,
        )
        .unwrap();

        assert_eq!(policy.stop_after, None);
        assert_eq!(policy.max_delay, None);
    }

    #[test]
    fn rejects_an_invalid_duration_string() {
        let result = serde_json::from_str::<RetryPolicy<Exponential, NoJitter>>(
            r#"{
                "backoff": { "kind": "exponential", "base": "eventually" },
                "jitter": { "kind": "none" }
            }"#,
        );

        assert!(result.is_err());
    }
}
