#![allow(clippy::type_complexity)]

#[cfg(all(feature = "iter", not(any(feature = "tokio", feature = "async-std"))))]
compile_error!("The 'iter' feature requires either the 'tokio' or 'async-std' feature");

pub mod backoff;
pub mod blocking;
pub mod executor;
pub mod jitter;
mod retry_policy;

#[cfg(feature = "iter")]
pub mod iter;

pub use backoff::{Backoff, Exponential, Fixed, Linear};
pub use executor::{until, until_ok, Mulligan};
pub use jitter::{Decorrelated, Equal, Full, Jitter, NoJitter};

/// Create an async retry stream with default settings (no delay, no limit).
#[cfg(feature = "iter")]
pub fn iter() -> iter::RetryStream<backoff::Fixed, jitter::NoJitter> {
    iter::RetryStream::new()
}
