mod exponential;

use std::time::Duration;

pub use exponential::Exponential;

pub trait Strategy {
    fn attempt(&self) -> u32;
    fn delay(&mut self) -> Duration;
}

pub struct Fixed {
    attempt: u32,
    dur: Duration,
}

impl Strategy for Fixed {
    fn attempt(&self) -> u32 {
        self.attempt
    }

    fn delay(&mut self) -> Duration {
        self.attempt += 1;
        self.dur
    }
}

pub struct Linear {
    attempt: u32,
    delay: Duration,
    max_delay: Option<Duration>,
}

impl Strategy for Linear {
    fn attempt(&self) -> u32 {
        self.attempt
    }
    fn delay(&mut self) -> Duration {
        let mut dur = self.delay * self.attempt;
        if let Some(max_wait) = self.max_delay {
            if max_wait < dur {
                dur = max_wait;
            }
        }
        self.attempt += 1;
        dur
    }
}
