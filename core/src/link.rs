use crate::signal::Stop;
use bytes::{Buf, BytesMut};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};

struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

pub(crate) struct Link {
    connection: Connection,
    addr: SocketAddr,
}

impl Connection {
    pub fn new(stream: TcpStream, buffer_size: usize) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(buffer_size),
        }
    }
}

impl Link {
    pub(crate) fn new(stream: TcpStream, addr: SocketAddr, buffer_size: usize) -> Self {
        Self {
            connection: Connection::new(stream, buffer_size),
            addr,
        }
    }

    pub(crate) async fn handle(&mut self, mut stop: Stop) -> io::Result<()> {
        todo!()
    }
}
