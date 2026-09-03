use std::time::Duration;

#[cfg(feature = "serde")]
#[derive(Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum BackoffKind {
    Fixed,
    Linear,
    Exponential,
}

#[cfg(feature = "serde")]
impl BackoffKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Linear => "linear",
            Self::Exponential => "exponential",
        }
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BackoffConfig {
    kind: BackoffKind,
    base: duration_string::DurationString,
}

#[cfg(feature = "serde")]
fn deserialize_backoff<'de, D>(deserializer: D, expected: BackoffKind) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let config = <BackoffConfig as serde::Deserialize>::deserialize(deserializer)?;
    if config.kind != expected {
        return Err(serde::de::Error::custom(format_args!(
            "expected `{}` backoff, found `{}`",
            expected.name(),
            config.kind.name()
        )));
    }

    Ok(config.base.into())
}

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
        deserialize_backoff(deserializer, BackoffKind::Fixed).map(Self::base)
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
        deserialize_backoff(deserializer, BackoffKind::Linear).map(Self::base)
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
        deserialize_backoff(deserializer, BackoffKind::Exponential).map(Self::base)
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
    fn deserializes_named_backoffs_with_duration_strings() {
        let fixed: Fixed = serde_json::from_str(r#"{"kind":"fixed","base":"250ms"}"#).unwrap();
        let linear: Linear = serde_json::from_str(r#"{"kind":"linear","base":"2s"}"#).unwrap();
        let exponential: Exponential =
            serde_json::from_str(r#"{"kind":"exponential","base":"1m"}"#).unwrap();

        assert_eq!(fixed.base(), Duration::from_millis(250));
        assert_eq!(linear.base(), Duration::from_secs(2));
        assert_eq!(exponential.base(), Duration::from_secs(60));
    }

    #[test]
    fn rejects_a_backoff_kind_that_does_not_match_the_target_type() {
        let result = serde_json::from_str::<Fixed>(r#"{"kind":"linear","base":"250ms"}"#);

        assert!(result.is_err());
    }
}
