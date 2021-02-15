use crate::signal::{Stop, Stopper};
use crate::toxic::{StreamDirection, Toxic};
use crate::toxics;
use crate::{proxy::ProxyConfig, toxic::ToxicKind};
use bytes::{Buf, Bytes, BytesMut};
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};
// downstream and upstream
#[derive(Debug)]
pub(crate) struct Link {
    config: ProxyConfig,
    toxics: Vec<Toxic>,
    upstream_addr: SocketAddr,
    direction: StreamDirection,
    stop: Stop,
    stopper: Stopper,
    disband_receiver: Option<oneshot::Receiver<Ends>>,
    // TODO: add additional left and right streams
}

type Ends = (
    FramedRead<OwnedReadHalf, BytesCodec>,
    FramedWrite<OwnedWriteHalf, BytesCodec>,
);

impl Link {
    pub(crate) fn new(
        upstream_addr: SocketAddr,
        direction: StreamDirection,
        toxics: Vec<Toxic>,
        config: ProxyConfig,
        stop: Stop,
    ) -> Self {
        let (stop, stopper) = stop.fork();
        let link = Link {
            // reader,
            // writer,
            config,
            toxics,
            upstream_addr,
            direction,
            stop,
            stopper,
            disband_receiver: None,
        };
        link
    }

    pub(super) fn establish(
        &mut self,
        mut reader: FramedRead<OwnedReadHalf, BytesCodec>,
        mut writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
    ) {
        // TODO: do we need to add one more read and write streams to both ends?

        let toxics = self.toxics.clone();

        let (disband_sender, disband_receiver) = oneshot::channel::<Ends>();
        self.disband_receiver = Some(disband_receiver);

        tokio::spawn(async move {
            if toxics.is_empty() {
                println!("no toxics, just connect both ends");
                let forward_res = forward(&mut reader, &mut writer).await;
                println!("no toxics forward ended");
                dbg!(forward_res);
            } else {
                let r2 = forward_read(&mut reader);
                let left = BufReader::new(r2);
            }
            let _ = disband_sender.send((reader, writer));
        });
    }

    /// Cuts all the streams, stops all the ToxicRunner tasks, returns the original
    /// stream and the sink at the two ends.
    pub(super) async fn disband(
        self,
    ) -> (
        FramedRead<OwnedReadHalf, BytesCodec>,
        FramedWrite<OwnedWriteHalf, BytesCodec>,
        Vec<Toxic>,
    ) {
        self.stopper.stop();
        let (reader, writer) = self
            .disband_receiver
            .expect("State error: Link already disbanded, or never established")
            .await
            .expect("Channel closed unexpectedly");
        let toxics = self.toxics;

        (reader, writer, toxics)
    }
}

impl PartialEq for Link {
    fn eq(&self, other: &Self) -> bool {
        self.upstream_addr == other.upstream_addr && self.direction == other.direction
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

    pub async fn run(
        &self,
        input: impl Stream<Item = Bytes>,
        output: impl Sink<Bytes>,
        stop: Stop,
    ) -> io::Result<()> {
        match self.toxic.kind {
            // TODO: avoid cloning toxic if possible
            ToxicKind::Noop => toxics::run_noop(self.toxic.clone(), input, output).await,
            ToxicKind::Latency {
                latency,
                jitter,
                buffer_size,
            } => {
                todo!()
            }
            ToxicKind::Timeout { timeout } => {
                todo!()
            }
            ToxicKind::Bandwidth { rate, buffer_size } => {
                todo!()
            }
            ToxicKind::SlowClose { delay } => {
                todo!()
            }
            ToxicKind::Slicer {
                average_size,
                size_variation,
                delay,
            } => {
                todo!()
            }
            ToxicKind::LimitData { bytes } => {
                todo!()
            }
        }
    }
}

async fn forward(
    reader: &mut FramedRead<OwnedReadHalf, BytesCodec>,
    writer: &mut FramedWrite<OwnedWriteHalf, BytesCodec>,
) -> io::Result<()> {
    while let Some(el) = reader.next().await {
        match el {
            Ok(chunk) => {
                if let Err(err) = writer.send(chunk.into()).await {
                    // writer channel closed
                    return Err(err);
                }
            }
            Err(err) => {
                // reader channel closed
                return Err(err);
            }
        }
    }
    Ok(())
}

async fn forward2(
    mut reader: std::pin::Pin<&mut impl Stream<Item = Bytes>>,
    mut writer: std::pin::Pin<&mut impl Sink<Bytes>>,
) -> io::Result<()> {
    while let Some(chunk) = reader.next().await {
        if let Err(err) = writer.send(chunk.into()).await {
            // writer channel closed
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "welp"));
        }
    }
    Ok(())
}

fn forward_read(
    reader: &mut FramedRead<OwnedReadHalf, BytesCodec>,
) -> FramedRead<&[u8], BytesCodec> {
    let buh: Vec<u8> = Vec::new();
    let cap = 8192;
    let reader2 = FramedRead::with_capacity(buh.as_slice(), BytesCodec::new(), cap);
    reader2
}

/*
fn forward_read(
    reader: &mut FramedRead<OwnedReadHalf, BytesCodec>,
) -> impl Stream<Item = Bytes> {
    use async_stream::stream;

    use futures::pin_mut;
    use futures::stream::StreamExt;

    let s = stream! {
        while let Some(el) = reader.next().await {
            if let Ok(chunk) = el {
                yield Into::<Bytes>::into(chunk);
            }
            break;
        }
    };
    s
}
 */
