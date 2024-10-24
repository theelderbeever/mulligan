use std::time::Duration;

use rand::Rng;

use super::Strategy;

pub struct Exponential {
    attempt: u32,
    base: Duration,
    max_delay: Option<Duration>,
    jitter: Option<Jitter>,
    previous: Option<Duration>,
}

impl Exponential {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn base(mut self, dur: Duration) -> Self {
        self.base = dur;
        self
    }
    pub fn max_delay(mut self, dur: Duration) -> Self {
        self.max_delay = Some(dur);
        self
    }
    pub fn jitter(mut self, kind: Jitter) -> Self {
        self.jitter = Some(kind);
        self
    }
}

impl Default for Exponential {
    fn default() -> Self {
        Self {
            attempt: 0,
            base: Duration::from_secs(1),
            max_delay: None,
            jitter: None,
            previous: None,
        }
    }
}

impl Strategy for Exponential {
    fn attempt(&self) -> u32 {
        self.attempt
    }
    #[cfg(feature = "tokio")]
    fn delay(&mut self) -> Duration {
        let dur = self.base * 2u32.pow(self.attempt);
        self.attempt += 1;
        match self.jitter {
            None => self.max_delay.map(|max| max.min(dur)).unwrap_or(dur),

            Some(Jitter::Full) => {
                let capped = self.max_delay.unwrap_or(dur).min(dur);
                rand::thread_rng().gen_range(Duration::from_micros(0)..=capped)
            }

            Some(Jitter::Equal) => {
                let capped = self.max_delay.unwrap_or(dur).min(dur);
                let half = capped / 2;
                half + rand::thread_rng().gen_range(Duration::from_micros(0)..=half)
            }

            Some(Jitter::Decorrelated) => {
                let previous = self.previous.unwrap_or(self.base);
                let next = rand::thread_rng().gen_range(self.base..=previous * 3);
                self.previous = Some(next);
                self.max_delay.map(|max| max.min(next)).unwrap_or(next)
            }
        }
    }
}

/// Different strategies for calculating retry delays with jitter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jitter {
    /// Full Jitter (AWS style) - random delay between 0 and the exponential backoff value
    /// Best for avoiding thundering herd problems
    Full,

    /// Equal Jitter - half exponential backoff, half random
    /// Good balance between retry timing consistency and collision avoidance
    Equal,

    /// Decorrelated Jitter - uses previous delay to calculate next one
    /// temp = min(max, random(base, previous * 3))
    /// Good for distributed systems to prevent synchronized retries
    Decorrelated,
}
