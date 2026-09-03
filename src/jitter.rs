use std::time::Duration;

use rand::Rng;

#[cfg(feature = "serde")]
#[derive(Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum JitterKind {
    None,
    Full,
    Equal,
    Decorrelated,
}

#[cfg(feature = "serde")]
impl JitterKind {
    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Full => "full",
            Self::Equal => "equal",
            Self::Decorrelated => "decorrelated",
        }
    }
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StatelessJitterConfig {
    kind: JitterKind,
}

#[cfg(feature = "serde")]
fn deserialize_stateless_jitter<'de, D>(
    deserializer: D,
    expected: JitterKind,
) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let config = <StatelessJitterConfig as serde::Deserialize>::deserialize(deserializer)?;
    if config.kind != expected {
        return Err(serde::de::Error::custom(format_args!(
            "expected `{}` jitter, found `{}`",
            expected.name(),
            config.kind.name()
        )));
    }

    Ok(())
}

pub trait Jitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration;
}

#[derive(Clone, Copy)]
pub struct NoJitter;

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for NoJitter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_stateless_jitter(deserializer, JitterKind::None).map(|()| Self)
    }
}

impl Jitter for NoJitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        max.map_or(delay, |max| max.min(delay))
    }
}

#[derive(Clone, Copy)]
pub struct Full;

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Full {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_stateless_jitter(deserializer, JitterKind::Full).map(|()| Self)
    }
}

impl Jitter for Full {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        let capped = max.map_or(delay, |max| max.min(delay));
        rand::thread_rng().gen_range(Duration::from_micros(0)..=capped)
    }
}

#[derive(Clone, Copy)]
pub struct Equal;

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Equal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_stateless_jitter(deserializer, JitterKind::Equal).map(|()| Self)
    }
}

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

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Decorrelated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct DecorrelatedConfig {
            kind: JitterKind,
            base: duration_string::DurationString,
        }

        let config = <DecorrelatedConfig as serde::Deserialize>::deserialize(deserializer)?;
        if config.kind != JitterKind::Decorrelated {
            return Err(serde::de::Error::custom(format_args!(
                "expected `decorrelated` jitter, found `{}`",
                config.kind.name()
            )));
        }

        Ok(Self::base(config.base.into()))
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

    use super::{Decorrelated, Full};

    #[test]
    fn deserializes_decorrelated_base_from_a_duration_string() {
        let jitter: Decorrelated =
            serde_json::from_str(r#"{"kind":"decorrelated","base":"750ms"}"#).unwrap();

        assert_eq!(jitter.base, Duration::from_millis(750));
        assert_eq!(jitter.previous, Duration::ZERO);
    }

    #[test]
    fn deserializes_a_named_stateless_jitter() {
        let _: Full = serde_json::from_str(r#"{"kind":"full"}"#).unwrap();
    }

    #[test]
    fn rejects_a_jitter_kind_that_does_not_match_the_target_type() {
        let result = serde_json::from_str::<Full>(r#"{"kind":"equal"}"#);

        assert!(result.is_err());
    }
}
