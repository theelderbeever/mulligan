use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_core::Stream;

use crate::backoff::{Backoff, Exponential, Fixed, Linear};
use crate::jitter::{Jitter, NoJitter};

macro_rules! retry_policy_builder {
    ($name:ident { $($extra_field:ident : $extra_type:ty = $extra_default:expr),* $(,)? }) => {
        impl $name<Fixed, NoJitter> {
            pub fn new() -> Self {
                Self {
                    current: 0,
                    stop_after: None,
                    backoff: Fixed::base(Duration::from_secs(0)),
                    jitterable: NoJitter,
                    max: None,
                    $($extra_field: $extra_default,)*
                }
            }
        }

        impl<Back: Backoff, Jit: Jitter> $name<Back, Jit> {
            /// Sets the maximum number of attempts before stopping.
            pub fn stop_after(mut self, attempts: u32) -> Self {
                self.stop_after = Some(attempts);
                self
            }

            /// Cap the maximum delay between retries.
            pub fn max_delay(mut self, dur: Duration) -> Self {
                self.max = Some(dur);
                self
            }

            /// Wait a fixed amount of time between each retry.
            pub fn fixed(self, dur: Duration) -> $name<Fixed, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Fixed::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Wait a linearly growing amount of time between each retry `base * attempt`.
            pub fn linear(self, dur: Duration) -> $name<Linear, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Linear::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Wait an exponentially growing amount of time between each retry `base * 2^attempt`.
            pub fn exponential(self, dur: Duration) -> $name<Exponential, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: Exponential::base(dur),
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Use a custom backoff strategy.
            pub fn backoff<B: Backoff>(self, backoff: B) -> $name<B, Jit> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff,
                    jitterable: self.jitterable,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Use a custom jitter strategy.
            pub fn jitter<J: Jitter>(self, jitter: J) -> $name<Back, J> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: jitter,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Random delay between 0 and the backoff value.
            pub fn full_jitter(self) -> $name<Back, crate::jitter::Full> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Full,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Random delay between backoff/2 and the backoff value.
            pub fn equal_jitter(self) -> $name<Back, crate::jitter::Equal> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Equal,
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            /// Decorrelated jitter: min(max, random(base, previous * 3)).
            pub fn decorrelated_jitter(self, base: Duration) -> $name<Back, crate::jitter::Decorrelated> {
                $name {
                    current: self.current,
                    stop_after: self.stop_after,
                    backoff: self.backoff,
                    jitterable: crate::jitter::Decorrelated::base(base),
                    max: self.max,
                    $($extra_field: self.$extra_field,)*
                }
            }

            fn calculate_delay(&mut self, attempt: u32) -> Duration {
                let delay = self.backoff.delay(attempt);
                self.jitterable.jitter(delay, self.max)
            }
        }
    };
}

pub struct RetryStream<Back: Backoff, Jit: Jitter> {
    current: u32,
    stop_after: Option<u32>,
    backoff: Back,
    jitterable: Jit,
    max: Option<Duration>,
    sleep: Option<Pin<Box<tokio::time::Sleep>>>,
}

retry_policy_builder!(RetryStream {
    sleep: Option<Pin<Box<tokio::time::Sleep>>> = None,
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
            this.sleep = Some(Box::pin(tokio::time::sleep(delay)));
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

pub struct RetryIter<Back: Backoff, Jit: Jitter> {
    current: u32,
    stop_after: Option<u32>,
    backoff: Back,
    jitterable: Jit,
    max: Option<Duration>,
}

retry_policy_builder!(RetryIter {});

impl<Back: Backoff, Jit: Jitter> Iterator for RetryIter<Back, Jit> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop_after.is_some_and(|max| self.current > max) {
            return None;
        }

        if self.current == 0 {
            self.current += 1;
            return Some(0);
        }

        let delay = self.calculate_delay(self.current - 1);
        std::thread::sleep(delay);

        let attempt = self.current;
        self.current += 1;
        Some(attempt)
    }
}

impl<Back: Backoff + Clone, Jit: Jitter + Clone> IntoIterator for &RetryIter<Back, Jit> {
    type Item = u32;
    type IntoIter = RetryIter<Back, Jit>;

    fn into_iter(self) -> Self::IntoIter {
        RetryIter {
            current: 0,
            stop_after: self.stop_after,
            backoff: self.backoff.clone(),
            jitterable: self.jitterable.clone(),
            max: self.max,
        }
    }
}
