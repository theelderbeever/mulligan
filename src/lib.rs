#![allow(clippy::type_complexity)]

pub mod backoff;
pub mod executor;
pub mod jitter;

#[cfg(feature = "iter")]
pub mod blocking;
#[cfg(feature = "iter")]
pub mod iter;

pub use executor::{until, until_ok};

/// Create an async retry stream with default settings (no delay, no limit).
#[cfg(feature = "iter")]
pub fn iter() -> iter::RetryStream<backoff::Fixed, jitter::NoJitter> {
    iter::RetryStream::new()
}
