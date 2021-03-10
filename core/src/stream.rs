use crate::signal::Stop;
use crate::socket::{ReadStream, WriteStream};
use bytes::{Bytes, BytesMut};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::io;
use std::pin::Pin;

use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

pub(crate) type Read = FramedRead<ReadStream, BytesCodec>;
pub(crate) type Write = FramedWrite<WriteStream, BytesCodec>;

pub(crate) async fn forward(
    reader: &mut Read,
    writer: &mut Write,
    stop: &mut Stop,
) -> io::Result<()> {
    while !stop.stop_received() {
        let maybe_res: Option<io::Result<BytesMut>> = tokio::select! {
            res = reader.next() => res,
            _ = stop.recv() => None
        };
        if let Some(res) = maybe_res {
            match res {
                Ok(chunk) => {
                    if let Err(_err) = writer.send(chunk.into()).await {
                        // writer closed
                        break;
                    }
                }
                Err(err) => {
                    // reader closed
                    return Err(err);
                }
            }
        } else {
            // stop signal received
            break;
        }
    }
    Ok(())
}

pub(crate) async fn forward_read(
    mut reader: Read,
    mut writer: Pin<&mut impl Sink<Bytes>>,
    stop: &mut Stop,
) -> io::Result<Read> {
    while !stop.stop_received() {
        let maybe_res: Option<io::Result<BytesMut>> = tokio::select! {
            res = reader.next() => res,
            _ = stop.recv() => None
        };
        if let Some(res) = maybe_res {
            match res {
                Ok(chunk) => {
                    if let Err(_err) = writer.send(chunk.into()).await {
                        // writer channel closed
                        break;
                    }
                }
                Err(err) => {
                    // reader i/o error
                    return Err(err);
                }
            }
        } else {
            // stop signal received
            break;
        }
    }
    Ok(reader)
}

pub(crate) async fn forward_write(
    mut reader: Pin<&mut impl Stream<Item = Bytes>>,
    mut writer: Write,
    stop: &mut Stop,
) -> io::Result<Write> {
    while !stop.stop_received() {
        let maybe_chunk = tokio::select! {
            res = reader.next() => res,
            _ = stop.recv() => None
        };
        if let Some(chunk) = maybe_chunk {
            if let Err(_) = writer.send(chunk.into()).await {
                // writer channel closed
                break;
            }
        } else {
            break;
        }
    }
    Ok(writer)
}
