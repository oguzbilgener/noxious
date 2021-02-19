use crate::signal::{Stop, Stopper};
use crate::stream::{forward, forward_read, forward_write, Read, Write};
use crate::toxic::{StreamDirection, Toxic};
use crate::toxics;
use crate::{proxy::ProxyConfig, toxic::ToxicKind};
use bytes::Bytes;
use futures::channel::mpsc as futures_mpsc;
use futures::StreamExt;
use futures::{Sink, Stream};
use std::io;
use std::net::SocketAddr;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::pin;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

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

    pub(super) fn establish(&mut self, mut reader: Read, mut writer: Write) -> JoinHandle<()> {
        let toxics = self.toxics.clone();

        let (disband_sender, disband_receiver) = oneshot::channel::<Ends>();
        self.disband_receiver = Some(disband_receiver);
        let direction = self.direction;
        println!("[{}] establish", direction);
        let mut stop = self.stop.clone();

        if toxics.is_empty() {
            tokio::spawn(async move {
                println!("[{}] no toxics, just connect both ends", direction);
                if !stop.stop_received() {
                    let forward_res = forward(&mut reader, &mut writer, &mut stop).await;
                    println!("[{}] no toxics forward ended", direction);
                    if let Err(err) = forward_res {
                        dbg!(err);
                    }
                }

                let _ = disband_sender.send((reader, writer));
            })
        } else {
            let (left_end_tx, left_end_rx) = futures_mpsc::channel::<Bytes>(1);
            let (right_end_tx, right_end_rx) = futures_mpsc::channel::<Bytes>(1);
            let (mut stop_read, read_stopper) = stop.fork();
            let (mut stop_write, write_stopper) = stop.fork();
            let should_wait_for_manual_close = toxics.iter().any(|toxic| toxic.kind.needs_closer());
            println!(
                "[{}] should wait for manual close? {}",
                direction, should_wait_for_manual_close
            );
            let close_read_join = tokio::spawn(async move {
                pin!(left_end_tx);
                let res = forward_read(reader, left_end_tx, &mut stop_read).await;
                println!("[{}] read task ended, {}", direction, &stop_read);
                // Speed up closing the underlying connection by closing the other channel,
                // unless we should wait for a toxic to yield explicitly.
                if !should_wait_for_manual_close || stop_read.stop_received() {
                    println!("[{}] stop the write immediately", direction);
                    write_stopper.stop();
                }
                res
            });
            let close_write_join = tokio::spawn(async move {
                pin!(right_end_rx);
                let res = forward_write(right_end_rx, writer, &mut stop_write).await;
                println!("[{}] write task ended, {}", direction, &stop_write);
                // Speed up closing the underlying connection by closing the other channel,
                // unless we should wait for a toxic to yield explicitly.
                if !should_wait_for_manual_close || stop_write.stop_received() {
                    println!("[{}] stop the read immediately", direction);
                    read_stopper.stop();
                }
                res
            });

            let stop = self.stop.clone();
            let join_handle = tokio::spawn(async move {
                let result: Result<(io::Result<Read>, io::Result<Write>), tokio::task::JoinError> =
                    tokio::try_join!(close_read_join, close_write_join);
                println!("joined two ends!");
                match result {
                    Ok((read_res, write_res)) => {
                        if let Ok(reader) = read_res {
                            if let Ok(writer) = write_res {
                                println!("[{}] disband ready", direction);
                                let _ = disband_sender.send((reader, writer));
                                return;
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
                let mut stop = stop.clone();
                tokio::spawn(async move {
                    let reader = prev_pipe_read_rx;
                    let mut runner = ToxicRunner::new(toxic);

                    let maybe_res = tokio::select! {
                        res = runner.run(reader, pipe_tx) => {
                            println!("pipe closed");
                            Some(res)
                        },
                        _ = stop.recv() => {
                            println!("stop recv'd");
                            None
                        }
                    };
                    if let Some(res) = maybe_res {
                        if let Err(err) = res {
                            println!("Got err from a toxic {:?}", err);
                        }
                    }
                });
                prev_pipe_read_rx = pipe_rx;
            }

            tokio::spawn(async move { prev_pipe_read_rx.map(Ok).forward(right_end_tx).await });

            join_handle
        }
    }

    /// Cuts all the streams, stops all the ToxicRunner tasks, returns the original
    /// stream and the sink at the two ends.
    pub(super) async fn disband(self) -> io::Result<(Read, Write)> {
        println!("[{}] disband, calling stopper.stop!", self.direction);
        self.stopper.stop();
        let (reader, writer) = self
            .disband_receiver
            .expect("State error: Link already disbanded, or never established")
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "already closed?"))?;

        Ok((reader, writer))
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
            toxic,
        }
    }

    pub async fn run(
        &mut self,
        input: impl Stream<Item = Bytes>,
        output: impl Sink<Bytes>,
    ) -> io::Result<()> {
        pin!(input);
        pin!(output);
        match self.toxic.kind {
            ToxicKind::Noop => toxics::run_noop(input, output).await,
            ToxicKind::Latency { latency, jitter } => {
                toxics::run_latency(input, output, latency, jitter).await
            }
            ToxicKind::Timeout { timeout } => toxics::run_timeout(input, output, timeout).await,
            ToxicKind::Bandwidth { rate } => toxics::run_bandwidth(input, output, rate).await,
            ToxicKind::SlowClose { delay } => {
                toxics::run_slow_close(input, output, delay).await
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
