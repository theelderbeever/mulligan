use std::time::Duration;

use rand::Rng;

pub trait Jitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration;
}

pub struct NoJitter;

impl Jitter for NoJitter {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        max.map_or(delay, |max| max.min(delay))
    }
}

pub struct Full;

impl Jitter for Full {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        let capped = max.map_or(delay, |max| max.min(delay));
        rand::thread_rng().gen_range(Duration::from_micros(0)..=capped)
    }
}

pub struct Equal;

impl Jitter for Equal {
    fn jitter(&mut self, delay: Duration, max: Option<Duration>) -> Duration {
        let capped = max.map_or(delay, |max| max.min(delay));
        rand::thread_rng().gen_range((capped / 2)..=capped)
    }
}

pub struct Decorrelated {
    base: Duration,
    previous: Duration,
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
