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

## Contributing

Formatting and linting hooks are run via `pre-commit` and will run prior to each commit. If the hooks fail they will reject the commit. The `end-of-file-fixer` and `trailing-whitespace` will automatically make the necessary fixes and you can just `git add ... && git commit -m ...` again immediately. The `fmt` and `clippy` lints will require your intervention.

If you _MUST_ bypass the commit hooks to get things on a branch you can `git commit --no-verify -m ...` to skip the hooks.

```
brew install pre-commit

pre-commit install
```

```yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      # - id: check-yaml
      - id: end-of-file-fixer
      - id: trailing-whitespace
  - repo: https://github.com/doublify/pre-commit-rust
    rev: v1.0
    hooks:
      - id: fmt
      - id: clippy
        args: [ --all-targets, --, -D, clippy::all ]
```

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
        .on_retry(|prev, attempts| {       // Run before each retry.
            println!("In the {}-th attempt, the returned result is {:?}.", attempts, prev);
            println!("Start next attempt");
        })
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
