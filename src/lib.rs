#[cfg(all(feature = "tokio", feature = "async-std"))]
compile_error!("Only one of 'tokio' or 'async-std' features can be enabled at a time");

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("Either 'tokio' or 'async-std' feature must be enabled");

pub mod strategy;

use std::{future::Future, time::Duration};

use strategy::Strategy;

#[derive(Default)]
pub struct Mulligan {
    stop_after: Option<u32>,
}

impl Mulligan {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn stop_after(mut self, attempts: u32) -> Self {
        self.stop_after = Some(attempts);
        self
    }
    pub async fn spawn<S, F, Fut, Args, T, E>(
        &self,
        strategy: &mut S,
        f: F,
        args: Args,
    ) -> Result<T, E>
    where
        S: Strategy + Send,
        F: Fn(Args) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, E>> + Send,
        T: Send + 'static,
        E: Send + 'static,
        Args: Clone + Send + 'static,
    {
        loop {
            let res = f(args.clone()).await;
            if self
                .stop_after
                .map(|max| strategy.attempt() >= max)
                .unwrap_or(false)
                | res.is_ok()
            {
                return res;
            }
            let sleep_for = strategy.delay();
            println!(
                "Attempt: {}, Sleep For: {:?}",
                strategy.attempt(),
                sleep_for
            );
            Self::sleep(sleep_for).await;
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
