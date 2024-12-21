#![allow(clippy::type_complexity)]

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("At least on of 'tokio' or 'async-std' feature must be enabled");

pub mod backoff;
pub mod jitter;

use std::{future::Future, marker::PhantomData, time::Duration};

pub use backoff::{Backoff, Exponential, Fixed, Linear};
pub use jitter::{Decorrelated, Equal, Full, Jitter, NoJitter};

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
pub fn until_ok<T, E>() -> Mulligan<T, E, impl Fn(&Result<T, E>) -> bool, NoJitter, Fixed> {
    until::<T, E, _>(|result: &Result<T, E>| result.is_ok())
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
pub fn until<T, E, Cond>(f: Cond) -> Mulligan<T, E, Cond, NoJitter, Fixed>
where
    Cond: Fn(&Result<T, E>) -> bool,
{
    Mulligan {
        stop_after: None,
        until: f,
        backoff: Fixed::base(Duration::from_secs(0)),
        jitterable: jitter::NoJitter,
        max: None,
        on_retry: None,
        _phantom: PhantomData,
    }
}

/// Not meant to be constructed directly. Use `mulligan::until_ok()` or `mulligan::until(...)` to construct.
pub struct Mulligan<T, E, Cond, Jit, Back>
where
    Cond: Fn(&Result<T, E>) -> bool,
    Jit: jitter::Jitter,
    Back: backoff::Backoff,
{
    stop_after: Option<u32>,
    until: Cond,
    backoff: Back,
    jitterable: Jit,
    max: Option<Duration>,
    on_retry: Option<Box<dyn Fn(&Result<T, E>, u32) + Send + Sync + 'static>>,
    _phantom: PhantomData<(T, E)>,
}

impl<T, E, Cond, Jit, Back> Mulligan<T, E, Cond, Jit, Back>
where
    Cond: Fn(&Result<T, E>) -> bool,
    Jit: jitter::Jitter,
    Back: backoff::Backoff,
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
    pub async fn retry<F, Fut>(mut self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let mut attempt: u32 = 0;
        loop {
            let res = f().await;
            if self.stop_after.map_or(false, |max| attempt >= max) | (self.until)(&res) {
                return res;
            }
            let delay = self.backoff.delay(attempt);
            let jittered = self.jitterable.jitter(delay, self.max);

            Self::sleep(jittered).await;

            if let Some(on_retry) = &self.on_retry {
                on_retry(&res, attempt);
            }

            attempt += 1;
        }
    }
    /// Sets the function to be called before each retry;
    /// it will not be called before the first execution.
    ///
    /// For the incoming function, the first parameter represents
    /// the result of the last execution, and the second parameter
    /// represents the number of times it has been executed.
    pub fn on_retry<F>(mut self, on_retry: F) -> Self
    where
        F: Fn(&Result<T, E>, u32) + Send + Sync + 'static,
    {
        self.on_retry = Some(Box::new(on_retry));
        self
    }
    /// Sets the maximum number of attempts to retry before stopping regardless of whether `until` condition has been met.
    pub fn stop_after(mut self, attempts: u32) -> Self {
        self.stop_after = Some(attempts);
        self
    }
    /// Adjust the backoff by the provided jitter strategy
    pub fn jitter<J>(self, jitter: J) -> Mulligan<T, E, Cond, J, Back>
    where
        J: jitter::Jitter,
    {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: self.backoff,
            jitterable: jitter,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Adjust the calculated backoff by choosing a random delay between 0 and the backoff value
    pub fn full_jitter(self) -> Mulligan<T, E, Cond, jitter::Full, Back> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: self.backoff,
            jitterable: jitter::Full,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Adjust the calculated backoff by choosing a random delay between backoff / 2 and the backoff value
    pub fn equal_jitter(self) -> Mulligan<T, E, Cond, jitter::Equal, Back> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: self.backoff,
            jitterable: jitter::Equal,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Adjust the calculated backoff by choosing a min(max_backoff, random(base_backoff, previous_backoff * 3))
    pub fn decorrelated_jitter(
        self,
        base: Duration,
    ) -> Mulligan<T, E, Cond, jitter::Decorrelated, Back> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: self.backoff,
            jitterable: jitter::Decorrelated::base(base),
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Delay by the calculated backoff strategy.
    pub fn backoff<B>(self, backoff: B) -> Mulligan<T, E, Cond, Jit, B>
    where
        B: Backoff,
    {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff,
            jitterable: self.jitterable,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Wait a fixed amount of time between each retry.
    pub fn fixed(self, dur: Duration) -> Mulligan<T, E, Cond, Jit, Fixed> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: Fixed::base(dur),
            jitterable: self.jitterable,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Wait a growing amount of time between each retry `base * attempt`
    pub fn linear(self, dur: Duration) -> Mulligan<T, E, Cond, Jit, Linear> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: Linear::base(dur),
            jitterable: self.jitterable,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Wait a growing amount of time between each retry `base * 2.pow(attempt)`
    pub fn exponential(self, dur: Duration) -> Mulligan<T, E, Cond, Jit, Exponential> {
        Mulligan {
            stop_after: self.stop_after,
            until: self.until,
            backoff: Exponential::base(dur),
            jitterable: self.jitterable,
            max: self.max,
            on_retry: self.on_retry,
            _phantom: PhantomData,
        }
    }
    /// Cap the maximum amount of time between retries even when the calculated backoff is larger.
    pub fn max_delay(mut self, dur: Duration) -> Self {
        self.max = Some(dur);
        self
    }

    #[cfg(feature = "tokio")]
    async fn sleep(dur: Duration) {
        tokio::time::sleep(dur).await;
    }
    #[cfg(all(feature = "async-std", not(feature = "tokio")))]
    async fn sleep(dur: Duration) {
        async_std::future::sleep(dur).await;
    }
}
