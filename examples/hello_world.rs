use std::time::Duration;

async fn this_errors(msg: &str) -> std::io::Result<()> {
    println!("{msg}");
    Err(std::io::Error::other("uh oh!"))
}

#[tokio::main()]
async fn main() {
    let hello = tokio::spawn(async move {
        mulligan::until_ok()
            .stop_after(5)
            .max_delay(Duration::from_secs(3))
            .full_jitter()
            .exponential(Duration::from_secs(1))
            .retry(|| async { this_errors("hello").await })
            .await
    });
    let world = tokio::spawn(async move {
        mulligan::until(|r| r.is_ok())
            .stop_after(10)
            .full_jitter()
            .fixed(Duration::from_secs(1))
            .retry(|| async { this_errors("world").await })
            .await
    });

    let retry = tokio::spawn(async move {
        mulligan::until_ok()
            .stop_after(10)
            .full_jitter()
            .fixed(Duration::from_millis(200))
            .on_retry(|res, attempt| { println!("[retry] start to call retry(): attempt = {}, prev = {:?}", attempt, res) })
            .retry(|| async { this_errors("[retry] running").await })
            .await
    });

    let _ = hello.await;
    let _ = world.await;
    let _ = retry.await;

    let _ = tokio::spawn(async move {
        mulligan::until_ok()
            .stop_after(3)
            .full_jitter()
            .fixed(Duration::from_millis(200))
            .on_retry(|res, attempt| { println!("[on_retry] start to call retry() again. In last attempt = {}, result = {:?}", attempt, res) })
            .retry(|| async { this_errors("[retry] call `.retry()` and failed").await })
            .await
    }).await;
}
