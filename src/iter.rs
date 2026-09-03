use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;

use crate::backoff::Backoff;
use crate::jitter::Jitter;
use crate::retry_policy::RetryPolicy;

type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

#[cfg(feature = "tokio")]
fn sleep(dur: Duration) -> SleepFuture {
    Box::pin(tokio::time::sleep(dur))
}

#[cfg(all(feature = "async-std", not(feature = "tokio")))]
fn sleep(dur: Duration) -> SleepFuture {
    Box::pin(async_std::task::sleep(dur))
}

/// An asynchronous sequence of attempts produced by a retry policy.
pub struct AsyncAttempts<Back: Backoff, Jit: Jitter> {
    current: u32,
    policy: RetryPolicy<Back, Jit>,
    sleep: Option<SleepFuture>,
}

impl<Back: Backoff, Jit: Jitter> AsyncAttempts<Back, Jit> {
    pub(crate) fn new(policy: RetryPolicy<Back, Jit>) -> Self {
        Self {
            current: 0,
            policy,
            sleep: None,
        }
    }
}

impl<Back: Backoff + Unpin, Jit: Jitter + Unpin> Stream for AsyncAttempts<Back, Jit> {
    type Item = u32;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.policy.should_stop(this.current) {
            return Poll::Ready(None);
        }

        if this.current == 0 {
            this.current += 1;
            return Poll::Ready(Some(0));
        }

        if this.sleep.is_none() {
            let delay = this.policy.calculate_delay(this.current - 1);
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

    use crate::retry;

    #[tokio::test]
    async fn stop_after_limits_retries_after_the_initial_attempt() {
        let attempts = retry().stop_after(3).attempts().collect::<Vec<_>>().await;

        assert_eq!(attempts, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn zero_retries_still_yields_the_initial_attempt() {
        let attempts = retry().stop_after(0).attempts().collect::<Vec<_>>().await;

        assert_eq!(attempts, vec![0]);
    }
}
