use std::time::Duration;

use rand::Rng;

pub trait Jitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration;
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct NoJitter;

impl Jitter for NoJitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        max.map_or(delay, |max| max.min(delay))
    }
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Full;

impl Jitter for Full {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        let capped = max.map_or(delay, |max| max.min(delay));
        rand::thread_rng().gen_range(Duration::from_micros(0)..=capped)
    }
}

#[derive(Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Equal;

impl Jitter for Equal {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        let capped = max.map_or(delay, |max| max.min(delay));
        rand::thread_rng().gen_range((capped / 2)..=capped)
    }
}

#[derive(Clone)]
pub struct Decorrelated {
    base: Duration,
    previous: Duration,
}

impl Default for Decorrelated {
    fn default() -> Self {
        Self::base(Duration::ZERO)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Decorrelated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration =
            <duration_string::DurationString as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::base(duration.into()))
    }
}

impl Decorrelated {
    pub fn base(dur: Duration) -> Self {
        Self {
            base: dur,
            previous: Duration::from_secs(0),
        }
    }
}

impl Jitter for Decorrelated {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        self.previous = delay; // TODO: Need to check if this is correct?
        let next = rand::thread_rng().gen_range(self.base..=self.previous * 3);
        max.map_or_else(|| next, |max| max.min(next))
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use std::time::Duration;

    use super::Decorrelated;

    #[test]
    fn deserializes_decorrelated_base_from_a_duration_string() {
        let jitter: Decorrelated = serde_json::from_str(r#""750ms""#).unwrap();

        assert_eq!(jitter.base, Duration::from_millis(750));
        assert_eq!(jitter.previous, Duration::ZERO);
    }
}
