use std::time::Duration;

use mulligan::{strategy::Exponential, Mulligan};

async fn this_errors(msg: String) -> std::io::Result<()> {
    println!("{msg}");
    Err(std::io::Error::other("uh oh!"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let hello = tokio::spawn(async move {
        let mut strategy = Exponential::new().max_delay(Duration::from_secs(3));
        Mulligan::new()
            .stop_after(10)
            .spawn(
                &mut strategy,
                move |msg| async move { this_errors(msg).await },
                "hello".to_string(),
            )
            .await
    });
    let world = tokio::spawn(async move {
        let mut strategy = Exponential::new().max_delay(Duration::from_secs(1));
        Mulligan::new()
            .stop_after(10)
            .spawn(
                &mut strategy,
                move |msg| async move { this_errors(msg).await },
                "world".to_string(),
            )
            .await
    });
    let _ = hello.await;
    let _ = world.await;
}
