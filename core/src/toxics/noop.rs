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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toxics::test_utils::*;

    #[tokio::test]
    async fn passthrough_once() {
        passthrough_test(|stream, sink| async move { run_noop(stream, sink).await }).await;
    }

    #[tokio::test]
    async fn drop_out_channel_first() {
        drop_out_channel_first_test(|stream, sink| async move { run_noop(stream, sink).await })
            .await;
    }
}
