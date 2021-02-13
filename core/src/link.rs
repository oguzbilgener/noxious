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

// TODO add, update, remove toxic events

// TODO: create two Links just like Shopify because Toxics can be added to both
// downstream and upstream
pub(crate) struct Link {
    config: ProxyConfig,
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    addr: SocketAddr,
}

impl Link {
    pub(crate) fn new(
        reader: OwnedReadHalf,
        writer: OwnedWriteHalf,
        addr: SocketAddr,
        config: ProxyConfig,
    ) -> Self {
        Self {
            reader,
            writer,
            config,
            addr,
        }
    }

    pub(crate) async fn handle(&mut self, mut stop: Stop) -> io::Result<()> {
        todo!()
        // TODO: can we implement the same thing without ToxicStubs?
    }
}
