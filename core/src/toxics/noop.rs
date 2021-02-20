use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;

pub async fn run_noop(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
) -> io::Result<()> {
    let _ = input.map(Ok).forward(output).await;
    Ok(())
}
