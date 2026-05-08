use std::time::Duration;

use futures::StreamExt;

#[tokio::main]
async fn main() {
    // Async — using StreamExt combinators
    println!("=== Async Stream (for_each) ===");
    mulligan::iter()
        .stop_after(4)
        .exponential(Duration::from_millis(100))
        .full_jitter()
        .max_delay(Duration::from_secs(1))
        .for_each(|attempt| async move {
            println!("  attempt {attempt}");
        })
        .await;

    // Async — using while let
    println!("\n=== Async Stream (while let) ===");
    let mut stream = mulligan::iter()
        .stop_after(3)
        .fixed(Duration::from_millis(200));
    while let Some(attempt) = stream.next().await {
        println!("  attempt {attempt}");
    }

    // Sync — for loop
    println!("\n=== Sync Iterator (for loop) ===");
    for attempt in mulligan::blocking::iter()
        .stop_after(3)
        .linear(Duration::from_millis(100))
    {
        println!("  attempt {attempt}");
    }
}
