use crate::signal::Stop;
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::io;
use tokio::pin;
use tokio::time::sleep;
use tokio::time::Duration;

/// The SlowClose prevents the proxy connection from closing until after a delay.
pub(crate) async fn run_slow_close(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    mut stop: Stop,
    delay: u64, // in millis
) -> io::Result<()> {
    pin!(input);
    pin!(output);
    let mut res: io::Result<()> = Ok(());
    while !stop.stop_received() {
        let maybe_chunk = tokio::select! {
            res = input.next() => res,
            _ = stop.recv() => None,
        };
        if let Some(chunk) = maybe_chunk {
            if let Err(_) = output.send(chunk).await {
                res = Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "Write channel closed",
                ));
                break;
            }
        } else {
            break;
        }
    }
    tracing::debug!("Slow close sleep for {}", delay);
    sleep(Duration::from_millis(delay)).await;
    tracing::debug!("Slow close closing {}", delay);
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toxics::test_utils::*;
    use tokio_test::assert_ok;

    #[tokio::test]
    async fn passthrough_once() {
        let (stop, _) = Stop::new();
        passthrough_test(|stream, sink| async move { run_slow_close(stream, sink, stop, 0).await })
            .await;
    }

    #[tokio::test]
    async fn drop_out_channel_first_0_delay() {
        let (stop, stopper) = Stop::new();

        let (in_stream, mut in_sink) = create_stream_sink();
        let (mut out_stream, out_sink) = create_stream_sink();
        let data = gen_random_bytes(32);
        let expected = Some(data.clone());
        let handle =
            tokio::spawn(async move { run_slow_close(in_stream, out_sink, stop, 0).await });

        assert_ok!(in_sink.send(data).await);
        assert_eq!(expected, out_stream.next().await);
        drop(out_stream);
        stopper.stop();
        assert_ok!(handle.await.unwrap());
    }
}
