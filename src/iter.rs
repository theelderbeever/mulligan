use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;

use crate::backoff::{Backoff, Exponential, Fixed, Linear};
use crate::jitter::{Jitter, NoJitter};
use crate::retry_policy::retry_policy_builder;

type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

#[cfg(feature = "tokio")]
fn sleep(dur: Duration) -> SleepFuture {
    Box::pin(tokio::time::sleep(dur))
}

#[cfg(all(feature = "async-std", not(feature = "tokio")))]
fn sleep(dur: Duration) -> SleepFuture {
    Box::pin(async_std::task::sleep(dur))
}

pub struct RetryStream<Back: Backoff, Jit: Jitter> {
    current: u32,
    stop_after: Option<u32>,
    backoff: Back,
    jitterable: Jit,
    max: Option<Duration>,
    sleep: Option<SleepFuture>,
}

retry_policy_builder!(RetryStream {
    sleep: Option<SleepFuture> = None,
});

impl<Back: Backoff + Unpin, Jit: Jitter + Unpin> Stream for RetryStream<Back, Jit> {
    type Item = u32;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.stop_after.is_some_and(|max| this.current > max) {
            return Poll::Ready(None);
        }

        if this.current == 0 {
            this.current += 1;
            return Poll::Ready(Some(0));
        }

        if this.sleep.is_none() {
            let delay = this.calculate_delay(this.current - 1);
            this.sleep = Some(sleep(delay));
        }

        match this.sleep.as_mut().unwrap().as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => {
                this.sleep = None;
                let attempt = this.current;
                this.current += 1;
                Poll::Ready(Some(attempt))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::RetryStream;

    #[tokio::test]
    async fn stop_after_limits_retries_after_the_initial_attempt() {
        let attempts = RetryStream::new().stop_after(3).collect::<Vec<_>>().await;

        assert_eq!(attempts, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn zero_retries_still_yields_the_initial_attempt() {
        let attempts = RetryStream::new().stop_after(0).collect::<Vec<_>>().await;

        assert_eq!(attempts, vec![0]);
    }
}
