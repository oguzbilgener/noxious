use super::run_noop;
use bytes::Bytes;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::convert::TryInto;
use std::io;
use std::time::Duration;
use tokio::pin;
use tokio::time::sleep;

const INTERVAL: u64 = 100;
const UNIT: usize = 100;

pub async fn run_bandwidth(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    rate: u64, // in KB/s
) -> io::Result<()> {
    if rate == 0 {
        return run_noop(input, output).await;
    }
    pin!(input);
    pin!(output);

    while let Some(chunk) = input.next().await {
        let chunk_len: u64 = chunk
            .len()
            .try_into()
            .expect("Could not convert chunk size from usize to u64");
        let mut to_sleep = Duration::from_nanos(
            (Duration::from_millis(chunk_len).as_nanos() / rate as u128)
                .try_into()
                .expect("chunk is too large"),
        );

        let mut chunk = chunk;
        let rate: usize = rate
            .try_into()
            .expect("Could not convert bandwidth rate from u64 to usize");

        // If the rate is low enough, split the packet up and send in 100 millisecond intervals
        while chunk.len() > rate * UNIT {
            sleep(Duration::from_millis(INTERVAL)).await;
            let to_send = chunk.split_to(UNIT);
            if let Err(_) = output.send(to_send).await {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "Write channel closed",
                ));
            }
            to_sleep -= Duration::from_millis(INTERVAL);
        }
        // sleep's granularity is 1ms
        if to_sleep.as_millis() > 0 {
            sleep(to_sleep).await;
        }
    }

    Ok(())
}
