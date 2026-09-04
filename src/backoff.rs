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
    #[serde(default)]
    multiplier: Option<f64>,
}

#[cfg(feature = "serde")]
fn deserialize_backoff_config<'de, D>(
    deserializer: D,
    expected: BackoffKind,
) -> Result<BackoffConfig, D::Error>
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

    Ok(config)
}

#[cfg(feature = "serde")]
fn deserialize_backoff<'de, D>(deserializer: D, expected: BackoffKind) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_backoff_config(deserializer, expected).map(|config| config.base.into())
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
pub struct Exponential {
    base: Duration,
    multiplier: f64,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Exponential {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_backoff_config(deserializer, BackoffKind::Exponential).map(|config| {
            Self::base(config.base.into()).multiplier(config.multiplier.unwrap_or(2.0))
        })
    }
}

impl Exponential {
    pub fn base(dur: Duration) -> Self {
        Self {
            base: dur,
            multiplier: 2.0,
        }
    }

    pub fn multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }
}

impl Backoff for Exponential {
    fn base(&self) -> Duration {
        self.base
    }
    fn delay(&self, attempt: u32) -> Duration {
        self.base.mul_f64(self.multiplier.powf(f64::from(attempt)))
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
        assert_eq!(exponential.delay(2), Duration::from_secs(240));
    }

    #[test]
    fn deserializes_a_configurable_exponential_multiplier() {
        let exponential: Exponential =
            serde_json::from_str(r#"{"kind":"exponential","base":"500ms","multiplier":1.5}"#)
                .unwrap();

        assert_eq!(exponential.delay(1), Duration::from_millis(750));
        assert_eq!(exponential.delay(2), Duration::from_millis(1125));
    }

    #[test]
    fn rejects_a_backoff_kind_that_does_not_match_the_target_type() {
        let result = serde_json::from_str::<Fixed>(r#"{"kind":"linear","base":"250ms"}"#);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Backoff, Exponential};

    #[test]
    fn uses_a_configurable_multiplier() {
        let exponential = Exponential::base(Duration::from_secs(1)).multiplier(1.5);

        assert_eq!(exponential.delay(0), Duration::from_secs(1));
        assert_eq!(exponential.delay(1), Duration::from_millis(1500));
        assert_eq!(exponential.delay(2), Duration::from_millis(2250));
    }
}
