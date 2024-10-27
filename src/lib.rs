#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!("Only one of 'tokio' or 'async-std' features can be enabled at a time");

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("Either 'tokio' or 'async-std' feature must be enabled");

use std::{future::Future, marker::PhantomData, time::Duration};

use rand::Rng;

pub fn until_ok<T, E>() -> Mulligan<T, E, impl Fn(&Result<T, E>) -> bool> {
    until(|result: &Result<T, E>| result.is_ok())
}

pub fn until<T, E, F>(f: F) -> Mulligan<T, E, F>
where
    F: Fn(&Result<T, E>) -> bool,
{
    Mulligan {
        stop_after: None,
        until: f,
        strategy: Strategy::Fixed(Duration::from_secs(0)),
        jitter: Jitter::None,
        max: None,
        _phantom: PhantomData,
    }
}
pub struct Mulligan<T, E, Cond>
where
    Cond: Fn(&Result<T, E>) -> bool,
{
    stop_after: Option<u32>,
    until: Cond,
    strategy: Strategy,
    jitter: Jitter,
    max: Option<Duration>,
    _phantom: PhantomData<(T, E)>,
}

impl<T, E, Cond> Mulligan<T, E, Cond>
where
    Cond: Fn(&Result<T, E>) -> bool,
{
    pub fn stop_after(&mut self, attempts: u32) -> &mut Self {
        self.stop_after = Some(attempts);
        self
    }
    pub fn no_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::None;
        self
    }
    pub fn full_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Full;
        self
    }
    pub fn equal_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Equal;
        self
    }
    pub fn decorrelated_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Decorrelated;
        self
    }
    pub fn fixed(&mut self, dur: Duration) -> &mut Self {
        self.strategy = Strategy::Fixed(dur);
        self
    }
    pub fn linear(&mut self, base: Duration) -> &mut Self {
        self.strategy = Strategy::Linear(base);
        self
    }
    pub fn exponential(&mut self, base: Duration) -> &mut Self {
        self.strategy = Strategy::Exponential(base);
        self
    }
    pub fn max_delay(&mut self, dur: Duration) -> &mut Self {
        self.max = Some(dur);
        self
    }
    pub async fn retry<F, Fut>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let mut previous = Duration::from_secs(0);
        let mut attempt: u32 = 0;
        loop {
            let res = f().await;
            if self.stop_after.map_or(false, |max| attempt >= max) | (self.until)(&res) {
                return res;
            }
            let delay = self.strategy.delay(attempt);
            let jittered = self
                .jitter
                .jitter(previous, self.strategy.base(), delay, self.max);

            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Attempt {} failed. Retry again in {}.",
                strategy.attempt(),
                sleep_for
            );
            Self::sleep(jittered).await;
            previous = delay;
            attempt += 1;
        }
    }

    #[cfg(feature = "tokio")]
    async fn sleep(dur: Duration) {
        tokio::time::sleep(dur).await;
    }
    #[cfg(feature = "async-std")]
    async fn sleep(dur: Duration) {
        async_std::future::sleep(dur).await;
    }
}

enum Strategy {
    Fixed(Duration),
    Linear(Duration),
    Exponential(Duration),
}

impl Strategy {
    pub fn delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed(dur) => *dur,
            Self::Linear(dur) => *dur * attempt,
            Self::Exponential(dur) => *dur * 2u32.pow(attempt),
        }
    }
    pub fn base(&self) -> Duration {
        match self {
            Self::Fixed(dur) | Self::Linear(dur) | Self::Exponential(dur) => *dur,
        }
    }
}

/// Different strategies for calculating retry delays with jitter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Jitter {
    None,

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

impl Jitter {
    pub fn jitter(
        &self,
        previous: Duration,
        base: Duration,
        delay: Duration,
        max: Option<Duration>,
    ) -> Duration {
        match self {
            Self::None => max.map_or(delay, |max| max.min(delay)),
            Self::Full => {
                let capped = max.map_or(delay, |max| max.min(delay));
                rand::thread_rng().gen_range(Duration::from_micros(0)..=capped)
            }
            Self::Equal => {
                let half = max.map_or(delay, |max| max.min(delay)) / 2;
                rand::thread_rng().gen_range(Duration::from_micros(0)..=half)
            }
            Self::Decorrelated => {
                let next = rand::thread_rng().gen_range(base..=previous * 3);
                max.map_or_else(|| next, |max| max.min(next))
            }
        }
    }
}
