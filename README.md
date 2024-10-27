# mulligan

## Example

```rust
use std::time::Duration;

async fn this_errors(msg: &str) -> std::io::Result<()> {
    println!("{msg}");
    Err(std::io::Error::other("uh oh!"))
}

mulligan::until_ok()
    .stop_after(5)
    .max_delay(Duration::from_secs(3))
    .full_jitter()
    .exponential(Duration::from_secs(1))
    .retry(|| async { this_errors("hello").await })
    .await

// Equivalent to just checking if Result::is_ok

mulligan::until(|res| res.is_ok())
    .stop_after(5)
    .max_delay(Duration::from_secs(3))
    .full_jitter()
    .exponential(Duration::from_secs(1))
    .retry(|| async { this_errors("hello").await })
    .await
```

