use crate::signal::{spawn_stoppable, Stop, Stopper};
use crate::toxic::{StreamDirection, Toxic};
use crate::toxics;
use crate::{proxy::ProxyConfig, toxic::ToxicKind};
use bytes::{Buf, Bytes, BytesMut};
use futures::channel::mpsc as futures_mpsc;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use futures_mpsc::SendError;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::pin;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
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
    ) -> JoinHandle<Option<()>> {
        // TODO: do we need to add one more read and write streams to both ends?

        let toxics = self.toxics.clone();

        let (disband_sender, disband_receiver) = oneshot::channel::<Ends>();
        self.disband_receiver = Some(disband_receiver);
        let direction = self.direction;

        if toxics.is_empty() {
            spawn_stoppable(self.stop.clone(), async move {
                println!("{} no toxics, just connect both ends", direction);
                let forward_res = forward(&mut reader, &mut writer).await;
                println!("{} no toxics forward ended", direction);
                if let Err(err) = forward_res {
                    dbg!(err);
                }
                let _ = disband_sender.send((reader, writer));
            })
        } else {
            let (left_end_tx, left_end_rx) = futures_mpsc::channel::<Bytes>(1);
            let (right_end_tx, right_end_rx) = futures_mpsc::channel::<Bytes>(1);
            let close_read_join = spawn_stoppable(self.stop.clone(), async move {
                pin!(left_end_tx);
                forward_read(reader, left_end_tx).await
            });
            let close_write_join = spawn_stoppable(self.stop.clone(), async move {
                pin!(right_end_rx);
                forward_write(right_end_rx, writer).await
            });

            let stop = self.stop.clone();
            let join_handle = spawn_stoppable(self.stop.clone(), async move {
                let result: Result<
                    (
                        Option<io::Result<FramedRead<OwnedReadHalf, BytesCodec>>>,
                        Option<io::Result<FramedWrite<OwnedWriteHalf, BytesCodec>>>,
                    ),
                    tokio::task::JoinError,
                > = tokio::try_join!(close_read_join, close_write_join);
                match result {
                    Ok((read_res, write_res)) => {
                        match (read_res, write_res) {
                            (None, _) | (_, None) => {
                                println!("welp...");
                                return;
                            }
                            (Some(read_res), Some(write_res)) => {
                                if let Ok(reader) = read_res {
                                    if let Ok(writer) = write_res {
                                        println!("{} disband ready", direction);
                                        let _ = disband_sender.send((reader, writer));
                                        return;
                                    }
                                }
                            }
                        }

                        panic!("read or write sub task failed");
                    }
                    Err(err) => {
                        panic!("read or write sub task failed {:?}", err);
                    }
                }
            });

            let mut prev_pipe_read_rx = left_end_rx;

            for toxic in toxics {
                let stop = stop.clone();
                // TODO: Get the desired channel buffer size for the toxic (in number of chunks)
                // This is 1024 for the Latency toxic and for other toxics, the channel is unbuffered.
                let (pipe_tx, pipe_rx) = futures_mpsc::channel::<Bytes>(1);
                spawn_stoppable(stop.clone(), async move {
                    // TODO: obey the Stop signal
                    let reader = prev_pipe_read_rx;
                    pin!(reader);
                    pin!(pipe_tx);
                    let res = ToxicRunner::new(toxic).run(reader, pipe_tx, stop).await;
                    if let Err(err) = res {
                        dbg!(err);
                    }
                });
                prev_pipe_read_rx = pipe_rx;
            }

            spawn_stoppable(stop.clone(), async move {
                prev_pipe_read_rx.map(Ok).forward(right_end_tx).await
            });

            join_handle
        }
    }

    /// Cuts all the streams, stops all the ToxicRunner tasks, returns the original
    /// stream and the sink at the two ends.
    pub(super) async fn disband(
        self,
    ) -> io::Result<(
        FramedRead<OwnedReadHalf, BytesCodec>,
        FramedWrite<OwnedWriteHalf, BytesCodec>,
        Vec<Toxic>,
    )> {
        self.stopper.stop();
        let (reader, writer) = self
            .disband_receiver
            .expect("State error: Link already disbanded, or never established")
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "already closed?"))?;

        Ok((reader, writer, self.toxics))
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
            ToxicKind::Latency { latency, jitter } => {
                todo!()
            }
            ToxicKind::Timeout { timeout } => {
                todo!()
            }
            ToxicKind::Bandwidth { rate } => {
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
                    println!("forward writer channel closed?");
                    return Err(err);
                }
            }
            Err(err) => {
                // reader channel closed
                println!("forward reader channel closed?");
                return Err(err);
            }
        }
    }
    println!("forward closed??");
    Ok(())
}

async fn forward_read(
    mut reader: FramedRead<OwnedReadHalf, BytesCodec>,
    mut writer: Pin<&mut impl Sink<Bytes>>,
) -> io::Result<FramedRead<OwnedReadHalf, BytesCodec>> {
    while let Some(el) = reader.next().await {
        match el {
            Ok(chunk) => {
                if let Err(_err) = writer.send(chunk.into()).await {
                    // writer channel closed
                    println!("fr write chan closed?");
                    return Ok(reader);
                }
            }
            Err(err) => {
                // reader channel closed
                println!("fr read chan closed?");
                return Err(err);
            }
        }
    }
    Ok(reader)
}

async fn forward_write(
    mut reader: Pin<&mut impl Stream<Item = Bytes>>,
    mut writer: FramedWrite<OwnedWriteHalf, BytesCodec>,
) -> io::Result<FramedWrite<OwnedWriteHalf, BytesCodec>> {
    while let Some(chunk) = reader.next().await {
        if let Err(_err) = writer.send(chunk.into()).await {
            // writer channel closed
            println!("fw write chan closed?");
            return Ok(writer);
        }
    }
    println!("fw read chan closed?");
    Ok(writer)
}
