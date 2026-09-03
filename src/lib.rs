#![allow(clippy::type_complexity)]

#[cfg(all(feature = "iter", not(any(feature = "tokio", feature = "async-std"))))]
compile_error!("The 'iter' feature requires either the 'tokio' or 'async-std' feature");

pub mod backoff;
mod blocking;
pub mod executor;
pub mod jitter;
mod retry_policy;

#[cfg(feature = "iter")]
mod iter;

pub use backoff::{Backoff, Exponential, Fixed, Linear};
pub use blocking::Attempts;
pub use executor::{until, until_ok, Mulligan};
#[cfg(feature = "iter")]
pub use iter::AsyncAttempts;
pub use jitter::{Decorrelated, Equal, Full, Jitter, NoJitter};
pub use retry_policy::{retry, RetryPolicy};
