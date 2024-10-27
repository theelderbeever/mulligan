#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!("Only one of 'tokio' or 'async-std' features can be enabled at a time");

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("Either 'tokio' or 'async-std' feature must be enabled");

use std::{future::Future, marker::PhantomData, time::Duration};

use rand::Rng;

/// Continues retrying the provided future until a successful result is obtained.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// async fn this_errors(msg: &str) -> std::io::Result<()> {
///     println!("{msg}");
///     Err(std::io::Error::other("uh oh!"))
/// }
///
/// # async fn example() {
/// mulligan::until_ok()
///     .stop_after(5)
///     .max_delay(Duration::from_secs(3))
///     .full_jitter()
///     .exponential(Duration::from_secs(1))
///     .retry(|| async { this_errors("hello").await })
///     .await;
/// # }
/// ```
pub fn until_ok<T, E>() -> Mulligan<T, E, impl Fn(&Result<T, E>) -> bool> {
    until(|result: &Result<T, E>| result.is_ok())
}

/// Continues retrying the provided future until a custom condition is met.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// async fn this_errors(msg: &str) -> std::io::Result<()> {
///     println!("{msg}");
///     Err(std::io::Error::other("uh oh!"))
/// }
///
/// # async fn example() {
/// mulligan::until(|res| res.is_ok())
///     .stop_after(5)
///     .max_delay(Duration::from_secs(3))
///     .full_jitter()
///     .exponential(Duration::from_secs(1))
///     .retry(|| async { this_errors("hello").await })
///     .await;
/// # }
/// ```
pub fn until<T, E, Cond>(f: Cond) -> Mulligan<T, E, Cond>
where
    Cond: Fn(&Result<T, E>) -> bool,
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

/// Not meant to be constructed directly. Use `mulligan::until_ok()` or `mulligan::until(...)` to construct.
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
    /// Retries a provided future until the stopping condition has been met. The default settings will
    /// retry forever with no delay between attempts. Backoff, Maximum Backoff, and Maximum Attempts
    /// can be configured with the other methods on the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    ///
    /// async fn this_errors(msg: &str) -> std::io::Result<()> {
    ///     println!("{msg}");
    ///     Err(std::io::Error::other("uh oh!"))
    /// }
    ///
    /// # async fn example() {
    /// mulligan::until_ok()
    ///     .retry(|| async { this_errors("hello").await })
    ///     .await;
    /// # }
    /// ```
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

            Self::sleep(jittered).await;
            previous = delay;
            attempt += 1;
        }
    }
    /// Sets the maximum number of attempts to retry before stopping regardless of whether `until` condition has been met.
    pub fn stop_after(&mut self, attempts: u32) -> &mut Self {
        self.stop_after = Some(attempts);
        self
    }
    /// Only delay by the calculated backoff strategy. This is the default. See `Mulligan::fixed`, `Mulligan::linear`, or `Mulligan::exponential`
    pub fn no_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::None;
        self
    }
    /// Adjust the calculated backoff by choosing a random delay between 0 and the backoff value
    pub fn full_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Full;
        self
    }
    /// Adjust the calculated backoff by choosing a random delay between backoff / 2 and the backoff value
    pub fn equal_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Equal;
        self
    }
    /// Adjust the calculated backoff by choosing a min(max_backoff, random(base_backoff, previous_backoff * 3))
    pub fn decorrelated_jitter(&mut self) -> &mut Self {
        self.jitter = Jitter::Decorrelated;
        self
    }
    /// Wait a fixed amount of time between each retry.
    pub fn fixed(&mut self, dur: Duration) -> &mut Self {
        self.strategy = Strategy::Fixed(dur);
        self
    }
    /// Wait a growing amount of time between each retry `base * attempt`
    pub fn linear(&mut self, base: Duration) -> &mut Self {
        self.strategy = Strategy::Linear(base);
        self
    }
    /// Wait a growing amount of time between each retry `base * 2.pow(attempt)`
    pub fn exponential(&mut self, base: Duration) -> &mut Self {
        self.strategy = Strategy::Exponential(base);
        self
    }
    /// Cap the maximum amount of time between retries even when the calculated backoff is larger.
    pub fn max_delay(&mut self, dur: Duration) -> &mut Self {
        self.max = Some(dur);
        self
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
    Full,
    Equal,
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
                let capped = max.map_or(delay, |max| max.min(delay));
                rand::thread_rng().gen_range((capped / 2)..=capped)
            }
            Self::Decorrelated => {
                let next = rand::thread_rng().gen_range(base..=previous * 3);
                max.map_or_else(|| next, |max| max.min(next))
            }
        }
    }
}
