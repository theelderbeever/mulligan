# mulligan

## Example

```rust
use std::time::Duration;

async fn this_errors(msg: String) -> std::io::Result<()> {
    println!("{msg}");
    Err(std::io::Error::other("uh oh!"))
}

mulligan::stop_if_ok()
    .stop_after(5)
    .exponential(
        Duration::from_secs(1),
        Some(Duration::from_secs(3)),
        Some(mulligan::Jitter::Full),
        || async move { this_errors("hello".to_string()).await },
    )
    .await
```