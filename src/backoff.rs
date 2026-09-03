use std::time::Duration;

pub trait Backoff {
    fn delay(&self, attempt: u32) -> Duration;
    fn base(&self) -> Duration;
}

#[derive(Clone, Copy)]
pub struct Fixed(Duration);

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Fixed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration =
            <duration_string::DurationString as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::base(duration.into()))
    }
}

impl Fixed {
    pub fn base(dur: Duration) -> Self {
        Self(dur)
    }
}

impl Backoff for Fixed {
    fn base(&self) -> Duration {
        self.0
    }
    fn delay(&self, _attempt: u32) -> Duration {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct Linear(Duration);

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Linear {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration =
            <duration_string::DurationString as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::base(duration.into()))
    }
}

impl Linear {
    pub fn base(dur: Duration) -> Self {
        Self(dur)
    }
}

impl Backoff for Linear {
    fn base(&self) -> Duration {
        self.0
    }
    fn delay(&self, attempt: u32) -> Duration {
        self.0 * attempt
    }
}

#[derive(Clone, Copy)]
pub struct Exponential(Duration);

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Exponential {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let duration =
            <duration_string::DurationString as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::base(duration.into()))
    }
}

impl Exponential {
    pub fn base(dur: Duration) -> Self {
        Self(dur)
    }
}

impl Backoff for Exponential {
    fn base(&self) -> Duration {
        self.0
    }
    fn delay(&self, attempt: u32) -> Duration {
        self.0 * 2u32.pow(attempt)
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use std::time::Duration;

    use super::{Backoff, Exponential, Fixed, Linear};

    #[test]
    fn deserializes_backoff_durations_from_strings() {
        let fixed: Fixed = serde_json::from_str(r#""250ms""#).unwrap();
        let linear: Linear = serde_json::from_str(r#""2s""#).unwrap();
        let exponential: Exponential = serde_json::from_str(r#""1m""#).unwrap();

        assert_eq!(fixed.base(), Duration::from_millis(250));
        assert_eq!(linear.base(), Duration::from_secs(2));
        assert_eq!(exponential.base(), Duration::from_secs(60));
    }
}
