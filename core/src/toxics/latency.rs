use crate::toxic::{Toxic, ToxicKind};
use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;
use rand;

pub async fn run_latency(
    toxic: Toxic,
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
) -> io::Result<()> {
    let (latency, jitter) = match toxic.kind {
        ToxicKind::Latency { latency, jitter } => (latency, jitter),
        _ => { panic!("invalid toxic kind given to latency runner {:?}", toxic.kind)}
    };
    let res = input.then(|chunk| async {
        // let wait =
        chunk
    }).map(Ok).forward(output).await;
    Ok(())
}
