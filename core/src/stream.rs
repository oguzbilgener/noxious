use crate::signal::Stop;
use bytes::{Bytes, BytesMut};
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::io;
use std::pin::Pin;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

pub(crate) type Read = FramedRead<OwnedReadHalf, BytesCodec>;
pub(crate) type Write = FramedWrite<OwnedWriteHalf, BytesCodec>;

pub(crate) async fn forward(
    reader: &mut Read,
    writer: &mut Write,
    mut stop: Stop,
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
                        // writer channel closed
                        println!("fr write chan closed?");
                        break;
                    }
                }
                Err(err) => {
                    // reader channel closed
                    println!("fr read chan closed?");
                    return Err(err);
                }
            }
        } else {
            break;
        }
    }
    // TODO: what happens when we drop the writer stream? the chain closes?
    Ok(())
}

pub(crate) async fn forward_read(
    mut reader: Read,
    mut writer: Pin<&mut impl Sink<Bytes>>,
    mut stop: Stop,
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
                        // println!("fr write chan closed?");
                        break;
                    }
                }
                Err(err) => {
                    // reader channel closed
                    // println!("fr read chan closed?");
                    return Err(err);
                }
            }
        } else {
            break;
        }
    }
    // TODO: what happens when we drop the writer stream? the chain closes?
    Ok(reader)
}

pub(crate) async fn forward_write(
    mut reader: Pin<&mut impl Stream<Item = Bytes>>,
    mut writer: Write,
    mut stop: Stop,
) -> io::Result<Write> {
    while !stop.stop_received() {
        let maybe_chunk = tokio::select! {
            res = reader.next() => res,
            _ = stop.recv() => None
        };
        if let Some(chunk) = maybe_chunk {
            if let Err(err) = writer.send(chunk.into()).await {
                // writer channel closed
                break;
            }
        } else {
            break;
        }
    }
    Ok(writer)
}
