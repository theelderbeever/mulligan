use std::time::Duration;

async fn this_errors(msg: String) -> std::io::Result<()> {
    println!("{msg}");
    Err(std::io::Error::other("uh oh!"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let hello = tokio::spawn(async move {
        mulligan::stop_if_ok()
            .stop_after(10)
            .exponential(
                Duration::from_secs(1),
                Some(Duration::from_secs(3)),
                Some(mulligan::Jitter::Full),
                || async move { this_errors("hello".to_string()).await },
            )
            .await
    });
    let world = tokio::spawn(async move {
        mulligan::stop_if_ok()
            .stop_after(10)
            .linear(
                Duration::from_secs(2),
                Some(Duration::from_secs(4)),
                || async move { this_errors("world".to_string()).await },
            )
            .await
    });
    let universe = tokio::spawn(async move {
        mulligan::stop_if_ok()
            .stop_after(10)
            .linear(
                Duration::from_secs(2),
                Some(Duration::from_secs(4)),
                || async move { this_errors("universe".to_string()).await },
            )
            .await
    });

    let _ = hello.await;
    let _ = world.await;
    let _ = universe.await;
}
