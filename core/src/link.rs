use crate::proxy::ProxyConfig;
use crate::signal::Stop;
use crate::toxic::{StreamDirection, Toxic};
use bytes::{Buf, BytesMut};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};
// downstream and upstream
#[derive(Debug)]
pub(crate) struct Link {
    config: ProxyConfig,
    pub(super) reader: FramedRead<OwnedReadHalf, BytesCodec>,
    pub(super) writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
    toxics: Vec<Toxic>,
    upstream_addr: SocketAddr,
    direction: StreamDirection,
    // TODO: add additional left and right streams
}

impl Link {
    pub(crate) fn new(
        reader: FramedRead<OwnedReadHalf, BytesCodec>,
        writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
        upstream_addr: SocketAddr,
        direction: StreamDirection,
        toxics: Vec<Toxic>,
        config: ProxyConfig,
    ) -> Self {
        let link = Link {
            reader,
            writer,
            config,
            toxics,
            upstream_addr,
            direction,
        };
        link
    }

    pub(super) fn establish(&mut self, mut stop: Stop) -> io::Result<()> {
        // TODO: do we need to add one more read and write streams to both ends?
        todo!()
    }

    /// Cuts all the streams, stops all the ToxicRunner tasks, returns the original
    /// stream and the sink at the two ends.
    pub(super) fn disband(
        self,
    ) -> (
        FramedRead<OwnedReadHalf, BytesCodec>,
        FramedWrite<OwnedWriteHalf, BytesCodec>,
        Vec<Toxic>,
    ) {
        let reader = self.reader;
        let writer = self.writer;
        let toxics = self.toxics;

        // TODO: implement this
        todo!();

        (reader, writer, toxics)
    }
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct ToxicRunner {
    toxic: Toxic,
}

impl ToxicRunner {
    pub fn new(toxic: Toxic) -> Self {
        ToxicRunner {
            // TODO: figure out the interrupt logic, i.e. when the Link or its streams is dropped,
            // cancel the runner tasks
            toxic,
        }
    }
}
