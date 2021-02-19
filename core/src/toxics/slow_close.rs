use bytes::Bytes;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio::time::sleep;

/// The SlowCloseToxic stops the TCP connection from closing until after a delay.
pub async fn run_slow_close(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    delay: u64, // in millis
) -> io::Result<()> {
    pin!(input);
    pin!(output);

    let mut res: io::Result<()> = Ok(());

    while let Some(chunk) = input.next().await {
        if let Err(_) = output.send(chunk).await {
            println!("done err but delaying by some time");
            res = Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "Write channel closed",
            ));
        }
    }
    println!("done ok but delaying by some time");

    sleep(Duration::from_millis(delay)).await;
    print!("now closing task!\n  ## ");

    res
}
