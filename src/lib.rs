#![allow(clippy::type_complexity)]

#[cfg(not(any(feature = "tokio", feature = "async-std")))]
compile_error!("At least on of 'tokio' or 'async-std' feature must be enabled");

pub mod backoff;
pub mod executor;
pub mod jitter;

pub use executor::{until, until_ok};
