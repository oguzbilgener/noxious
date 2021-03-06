use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;
use tokio::time::Duration;
use tokio::time::sleep;
use tokio::{io::AsyncWriteExt, pin};

/// The TimeoutToxic stops any data from flowing through, and will close the connection after a timeout.
/// If the timeout is set to 0, then the connection will not be closed.
pub async fn run_timeout(
    input: impl Stream<Item = Bytes>,
    _output: impl Sink<Bytes>,
    timeout: u64, // in millis
) -> io::Result<()> {
    let mut drain = tokio::io::sink();
    if timeout == 0 {
        pin!(input);
        // Drain the input until it's closed
        while let Some(chunk) = input.next().await {
            drain.write(&chunk).await?;
        }
    } else {
        input
            .take_until(sleep(Duration::from_millis(timeout)))
            .fold((), |_, _| async move { () })
            .await;
    }

    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("timeout after {}ms", timeout),
    ))
}
