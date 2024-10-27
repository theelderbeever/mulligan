# mulligan

A flexible retry library for Rust async operations with configurable backoff strategies and jitter.

[![Crates.io](https://img.shields.io/crates/v/mulligan.svg)](https://crates.io/crates/mulligan)
[![Documentation](https://docs.rs/mulligan/badge.svg)](https://docs.rs/mulligan)

`mulligan` provides a fluent API for retrying async operations with customizable retry policies, backoff strategies, and jitter. It supports both `tokio` and `async-std` runtimes.

## Features

- Multiple backoff strategies:
  - Fixed delay
  - Linear backoff
  - Exponential backoff
- Configurable jitter options:
  - Full jitter
  - Equal jitter
  - Decorrelated jitter
- Maximum retry attempts
- Maximum delay caps
- Custom retry conditions
- Async runtime support:
  - `tokio` (via `tokio` feature)
  - `async-std` (via `async-std` feature)

## Quick Start

```rust
use std::time::Duration;

async fn fallible_operation(msg: &str) -> std::io::Result<()> {
    // Your potentially failing operation here
    Err(std::io::Error::other(msg))
}

#[tokio::main]
async fn main() {
    let result = mulligan::until_ok()
        .stop_after(5)                     // Try up to 5 times
        .max_delay(Duration::from_secs(3)) // Cap maximum delay at 3 seconds
        .exponential(Duration::from_secs(1)) // Use exponential backoff
        .full_jitter()                     // Add randomized jitter
        .retry(|| async {
            fallible_operation("connection failed").await
        })
        .await;
}
```

Alternatively, you may provide a custom stopping condition. `mulligan::until_ok()` is equivalent to the custom stopping condition shown below.

```rust
#[tokio::main]
async fn main() {
    let result = mulligan::until(|res| res.is_ok())
        .stop_after(5)                     // Try up to 5 times
        .max_delay(Duration::from_secs(3)) // Cap maximum delay at 3 seconds
        .exponential(Duration::from_secs(1)) // Use exponential backoff
        .full_jitter()                     // Add randomized jitter
        .retry(|| async {
            fallible_operation("connection failed").await
        })
        .await;
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mulligan = { version = "0.1", features = ["tokio"] } # or ["async-std"]
```

Note: You must enable either the `tokio` or `async-std` feature, but not both.