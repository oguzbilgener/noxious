use bytes::Bytes;
use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    Future,
};
use futures::{SinkExt, StreamExt};
use std::io;
use tokio_test::assert_ok;

pub(crate) fn create_stream_sink() -> (Receiver<Bytes>, Sender<Bytes>) {
    let (tx, rx) = channel(1);
    (rx, tx)
}

pub(crate) fn gen_random_bytes(length: usize) -> Bytes {
    let range = 0..length;
    range
        .map(|_| rand::random::<u8>())
        .collect::<Vec<u8>>()
        .into()
}

pub(crate) async fn passthrough_test<F>(
    make_handle: impl FnOnce(Receiver<Bytes>, Sender<Bytes>) -> F,
) where
    F: Future<Output = io::Result<()>> + 'static + Send + Sync,
{
    let (in_stream, mut in_sink) = create_stream_sink();
    let (mut out_stream, out_sink) = create_stream_sink();
    let data = gen_random_bytes(32);
    let expected = Some(data.clone());
    let handle = tokio::spawn(make_handle(in_stream, out_sink));

    assert_ok!(in_sink.send(data).await);
    drop(in_sink);
    assert_ok!(handle.await.unwrap());
    assert_eq!(expected, out_stream.next().await);
}

pub(crate) async fn drop_out_channel_first_test<F>(
    make_handle: impl FnOnce(Receiver<Bytes>, Sender<Bytes>) -> F,
) where
    F: Future<Output = io::Result<()>> + 'static + Send + Sync,
{
    let (in_stream, mut in_sink) = create_stream_sink();
    let (out_stream, out_sink) = create_stream_sink();
    let data = gen_random_bytes(32);
    let handle = tokio::spawn(make_handle(in_stream, out_sink));

    assert_ok!(in_sink.send(data).await);
    drop(out_stream);
    let _ = handle.await;
}
