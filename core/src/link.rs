use crate::proxy::ProxyConfig;
use crate::signal::Stop;
use crate::toxic::StreamDirection;
use bytes::{Buf, BytesMut};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

// TODO add, update, remove toxic events

// TODO: create two Links just like Shopify because Toxics can be added to both
// downstream and upstream
#[derive(Debug)]
pub(crate) struct Link {
    config: ProxyConfig,
    reader: FramedRead<OwnedReadHalf, BytesCodec>,
    writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
    addr: SocketAddr,
}

impl Link {
    pub(crate) fn new(
        reader: FramedRead<OwnedReadHalf, BytesCodec>,
        writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
        addr: SocketAddr,
        config: ProxyConfig,
    ) -> Self {
        let link = Link {
            reader,
            writer,
            config,
            addr,
        };
        link
    }

    pub(crate) fn establish(&mut self, mut stop: Stop) -> io::Result<()> {
        todo!()
        // TODO: can we implement the same thing without ToxicStubs?
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        todo!()
    }
}
