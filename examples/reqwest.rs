use log::{debug, error, info, warn};
use std::time::Duration;

/// This example demonstrates using mulligan to retry HTTP requests to httpbin.org
///
/// It shows several scenarios:
/// 1. Retrying a request that always fails (status endpoint)
/// 2. Retrying until we get a successful response
/// 3. Custom retry conditions based on HTTP status codes
/// 4. Reusing a reqwest::Client across multiple retries by capturing it in closures
///
/// Note: httpbin's `/status/:code` endpoint returns the specified HTTP status code
///
/// Run with: cargo run --example reqwest
/// For more verbose output: RUST_LOG=debug cargo run --example reqwest
/// For minimal output: RUST_LOG=warn cargo run --example reqwest
async fn fetch_with_status(
    client: &reqwest::Client,
    status_code: u16,
) -> Result<reqwest::Response, reqwest::Error> {
    let url = format!("https://httpbin.org/status/{}", status_code);
    debug!("Requesting: {}", url);
    client.get(&url).send().await
}

async fn fetch_json(client: &reqwest::Client) -> Result<serde_json::Value, reqwest::Error> {
    debug!("Requesting: https://httpbin.org/json");
    let response = client.get("https://httpbin.org/json").send().await?;
    let json = response.json::<serde_json::Value>().await?;
    Ok(json)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    // Create a single client to reuse across all requests
    let client = reqwest::Client::new();

    info!("Example 1: Retry a failing request (503 Service Unavailable)");

    let result = mulligan::until_ok()
        .stop_after(5)
        .max_delay(Duration::from_secs(3))
        .exponential(Duration::from_millis(500))
        .full_jitter()
        .after_attempt(|res, attempt| {
            if let Err(e) = res {
                debug!("Attempt {} failed: {}", attempt + 1, e);
            }
        })
        .execute(async || fetch_with_status(&client, 503).await)
        .await;

    match result {
        Ok(response) => info!("Success with status: {}", response.status()),
        Err(e) => warn!("Failed after all retries: {}", e),
    }

    info!("Example 2: Retry until successful (200 OK)");

    let result = mulligan::until_ok()
        .stop_after(3)
        .fixed(Duration::from_millis(500))
        .equal_jitter()
        .execute(async || fetch_with_status(&client, 200).await)
        .await;

    match result {
        Ok(response) => info!("Success with status: {}", response.status()),
        Err(e) => error!("Failed: {}", e),
    }

    info!("Example 3: Custom retry condition (retry only on 5xx errors)");

    let result = mulligan::until(
        |res: &Result<reqwest::Response, reqwest::Error>| match res {
            Ok(response) => {
                let status = response.status();
                !status.is_server_error()
            }
            Err(_) => false,
        },
    )
    .stop_after(4)
    .linear(Duration::from_millis(300))
    .full_jitter()
    .after_attempt(|res, attempt| match res {
        Ok(response) => {
            let status = response.status();
            if status.is_server_error() {
                debug!(
                    "Attempt {}: Got 5xx error ({}), retrying...",
                    attempt + 1,
                    status
                );
            }
        }
        Err(e) => debug!("Attempt {}: Network error: {}", attempt + 1, e),
    })
    .execute(async || fetch_with_status(&client, 500).await)
    .await;

    match result {
        Ok(response) => info!("Final status: {}", response.status()),
        Err(e) => warn!("Failed: {}", e),
    }

    info!("Example 4: Fetching JSON with retry");

    let result = mulligan::until_ok()
        .stop_after(3)
        .exponential(Duration::from_millis(200))
        .decorrelated_jitter(Duration::from_millis(100))
        .max_delay(Duration::from_secs(2))
        .execute(async || fetch_json(&client).await)
        .await;

    match result {
        Ok(json) => info!(
            "Successfully fetched JSON: {}",
            serde_json::to_string_pretty(&json)?
        ),
        Err(e) => error!("Failed to fetch JSON: {}", e),
    }

    Ok(())
}
