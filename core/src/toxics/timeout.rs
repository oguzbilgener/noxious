use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;
use tokio::time::sleep;
use tokio::time::Duration;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toxics::test_utils::*;
    use futures::{SinkExt, StreamExt};
    use tokio::time::{pause, resume};
    use tokio_test::{assert_err, assert_ok};

    async fn test_timeout(timeout: u64) {
        let (in_stream, mut in_sink) = create_stream_sink();
        let (mut out_stream, out_sink) = create_stream_sink();
        let handle = tokio::spawn(run_timeout(in_stream, out_sink, timeout));

        assert_ok!(in_sink.send(gen_random_bytes(32)).await);
        assert_ok!(in_sink.send(gen_random_bytes(32)).await);
        drop(in_sink);
        let res = handle.await.unwrap();
        assert_err!(res);
        assert_eq!(None, out_stream.next().await);
    }

    #[tokio::test]
    async fn dumps_data() {
        let timeout = 1u64;
        test_timeout(timeout).await;
    }

    #[tokio::test]
    async fn resolves_when_time_paused() {
        let timeout = 5000u64;
        pause();
        test_timeout(timeout).await;
        resume();
    }

    #[tokio::test]
    async fn timeout_0_resolves_when_time_paused() {
        let timeout = 0u64;
        pause();
        test_timeout(timeout).await;
        resume();
    }
}
