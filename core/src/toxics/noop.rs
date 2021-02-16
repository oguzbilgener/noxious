use crate::toxic::Toxic;
use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;

pub async fn run_noop(
    toxic: Toxic,
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
) -> io::Result<()> {
    println!("[{}] run noop", toxic.direction);
    let res = input.map(Ok).forward(output).await;
    println!("[{}] forward ended", toxic.direction);
    Ok(())
}
