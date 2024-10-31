use std::time::Duration;

pub trait Backoff {
    fn delay(&self, attempt: u32) -> Duration;
    fn base(&self) -> Duration;
}

pub struct Fixed(Duration);

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

pub struct Linear(Duration);

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

pub struct Exponential(Duration);

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
