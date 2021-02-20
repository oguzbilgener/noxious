use crate::signal::Stop;
use bytes::Bytes;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio::time::sleep;

/// The SlowCloseToxic stops the TCP connection from closing until after a delay.
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
                println!("SLOW {} done err bc send err", delay);
                res = Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "Write channel closed",
                ));
            }
        } else {
            break;
        }
    }

    println!(
        "SLOW {} done ({}) but delaying by {}ms",
        delay, &stop, delay
    );

    sleep(Duration::from_millis(delay)).await;
    print!("after {}ms, closing slow close!\n##\n", delay);

    res
}
