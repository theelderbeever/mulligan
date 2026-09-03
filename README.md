# mulligan

A flexible retry library for Rust async operations with configurable backoff strategies and jitter.

[![Crates.io](https://img.shields.io/crates/v/mulligan.svg)](https://crates.io/crates/mulligan)
[![Documentation](https://docs.rs/mulligan/badge.svg)](https://docs.rs/mulligan)

`mulligan` provides a fluent API for retrying async operations with customizable retry policies, backoff strategies, and jitter. It supports the `tokio` runtime.

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

### Releases

Releases are prepared by [release-plz](https://release-plz.dev/) from Conventional Commit messages. Pull request titles must use the Conventional Commits format (for example, `fix: handle a zero retry limit`, `feat: add a backoff strategy`, or `feat!: remove a public API`). Configure GitHub to use the pull request title as the default squash-merge commit message so that release-plz can calculate the correct SemVer bump.

To release:

1. Release-plz will open or update a release PR after changes land on `main`. You can also run the **Release** workflow manually from `main`. The PR contains the version bump and changelog generated from the commits since the last crates.io release.
2. Review and merge that release PR.
3. Update local `main`, then cut and push a branch whose name exactly matches the version in `Cargo.toml`:

   ```shell
   git switch main
   git pull --ff-only
   git switch -c release/0.7.0
   git push origin release/0.7.0
   ```

The branch push runs formatting, linting, tests, and a package dry run. It then asks release-plz to create the `v<version>` tag and GitHub release and publish that version to crates.io. The job fails unless the branch is named `release/<Cargo.toml-version>` and points at a commit already on `main`.

Repository setup required once:

- In GitHub Actions settings, allow workflows to create pull requests.
- Add a crates.io trusted publisher for crate `mulligan`, repository `theelderbeever/mulligan`, workflow `release.yaml`, and environment `crates-io`. The publish job uses OIDC, so no long-lived `CARGO_REGISTRY_TOKEN` secret is needed.
- Protect `main` and `release/*` branches. Optionally require approval on the `crates-io` GitHub environment for a final manual publishing gate.

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
        .stop_after(5)                     // Retry up to 5 times after the initial attempt
        .max_delay(Duration::from_secs(3)) // Cap maximum delay at 3 seconds
        .exponential(Duration::from_secs(1)) // Use exponential backoff
        .full_jitter()                     // Add randomized jitter
        .execute(async {
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
        .stop_after(5)                     // Retry up to 5 times after the initial attempt
        .max_delay(Duration::from_secs(3)) // Cap maximum delay at 3 seconds
        .exponential(Duration::from_secs(1)) // Use exponential backoff
        .full_jitter()                     // Add randomized jitter
        .after_attempt(|prev, attempts| {       // Run before each retry.
            println!("In the {}-th attempt, the returned result is {:?}.", attempts, prev);
            println!("Start next attempt");
        })
        .execute(async {
            fallible_operation("connection failed").await
        })
        .await;
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mulligan = { version = "0.1", features = ["tokio"] }
```
