use std::time::Duration;

use futures::StreamExt;

#[tokio::main]
async fn main() {
    // Async — initial attempt followed by up to four retries
    println!("=== Async attempts (for_each) ===");
    mulligan::retry()
        .stop_after(4)
        .exponential(Duration::from_millis(100))
        .full_jitter()
        .max_delay(Duration::from_secs(1))
        .attempts()
        .for_each(|attempt| async move {
            println!("  attempt {attempt}");
        })
        .await;

    // Async — initial attempt followed by up to three retries
    println!("\n=== Async attempts (while let) ===");
    let mut attempts = mulligan::retry()
        .stop_after(3)
        .fixed(Duration::from_millis(200))
        .attempts();
    while let Some(attempt) = attempts.next().await {
        println!("  attempt {attempt}");
    }

    // Sync — initial attempt followed by up to three retries
    println!("\n=== Sync Iterator (for loop) ===");
    for attempt in mulligan::retry()
        .stop_after(3)
        .linear(Duration::from_millis(100))
        .attempts_sync()
    {
        println!("  attempt {attempt}");
    }
}
