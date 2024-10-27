#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!("Only one of 'tokio' or 'async-std' features can be enabled at a time");

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("Either 'tokio' or 'async-std' feature must be enabled");

pub mod strategy;

use std::{future::Future, marker::PhantomData, time::Duration};

pub use strategy::Jitter;
use strategy::{Exponential, Fixed, Linear, Strategy};

pub fn stop_if_ok<T, E>() -> Mulligan<T, E, impl Fn(&Result<T, E>) -> bool> {
    stop_if(|result: &Result<T, E>| result.is_ok())
}

pub fn stop_if<T, E, F>(f: F) -> Mulligan<T, E, F>
where
    F: Fn(&Result<T, E>) -> bool,
{
    Mulligan {
        stop_after: None,
        stop_if: f,
        _phantom: PhantomData,
    }
}
pub struct Mulligan<T, E, Cond>
where
    Cond: Fn(&Result<T, E>) -> bool,
{
    stop_after: Option<u32>,
    stop_if: Cond,
    _phantom: PhantomData<(T, E)>,
}

impl<T, E, Cond> Mulligan<T, E, Cond>
where
    Cond: Fn(&Result<T, E>) -> bool,
{
    pub fn stop_after(mut self, attempts: u32) -> Self {
        self.stop_after = Some(attempts);
        self
    }
    async fn retry<S, F, Fut>(&self, strategy: &mut S, f: F) -> Result<T, E>
    where
        S: Strategy + Send,
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        loop {
            let res = f().await;
            if self
                .stop_after
                .map(|max| strategy.attempt() >= max)
                .unwrap_or(false)
                | (self.stop_if)(&res)
            {
                return res;
            }
            let sleep_for = strategy.delay();
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Attempt {} failed. Retry again in {}.",
                strategy.attempt(),
                sleep_for
            );
            Self::sleep(sleep_for).await;
        }
    }
    pub async fn exponential<F, Fut>(
        &self,
        base: Duration,
        max_delay: Option<Duration>,
        jitter: Option<Jitter>,
        f: F,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let mut strategy = Exponential::new_with_values(base, max_delay, jitter);
        self.retry(&mut strategy, f).await
    }
    pub async fn linear<F, Fut>(
        &self,
        base: Duration,
        max_delay: Option<Duration>,
        f: F,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let mut strategy = Linear::new_with_values(base, max_delay);
        self.retry(&mut strategy, f).await
    }
    pub async fn fixed<F, Fut>(&self, dur: Duration, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
    {
        let mut strategy = Fixed::new_with_values(dur);
        self.retry(&mut strategy, f).await
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
