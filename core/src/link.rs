use crate::{
    proxy::ProxyConfig,
    signal::{Close, Closer, Stop, Stopper},
    state::{ToxicState, ToxicStateHolder},
    stream::{forward, forward_read, forward_write, Read, Write},
    toxic::ToxicKind,
    toxic::{StreamDirection, Toxic},
    toxics,
};
use bytes::Bytes;
use futures::channel::mpsc as futures_mpsc;
use futures::StreamExt;
use futures::{Sink, Stream};
use rand::{distributions::Standard, rngs::StdRng, Rng, SeedableRng};
use std::net::SocketAddr;
use std::{io, sync::Arc};
use tokio::pin;
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub(crate) struct Link {
    config: ProxyConfig,
    upstream_addr: SocketAddr,
    direction: StreamDirection,
    stop: Stop,
    stopper: Stopper,
    disband_receiver: Option<oneshot::Receiver<Ends>>,
}

type Ends = (Read, Write);

impl Link {
    pub(crate) fn new(
        upstream_addr: SocketAddr,
        direction: StreamDirection,
        config: ProxyConfig,
        stop: Stop,
    ) -> Self {
        let (stop, stopper) = stop.fork();
        let link = Link {
            config,
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
        reader: Read,
        writer: Write,
        toxics: Vec<Toxic>,
        toxic_state_holder: Option<Arc<ToxicStateHolder>>,
    ) -> JoinHandle<()> {
        let (disband_sender, disband_receiver) = oneshot::channel::<Ends>();
        self.disband_receiver = Some(disband_receiver);
        if toxics.is_empty() {
            self.forward_direct(reader, writer, disband_sender)
        } else {
            self.setup_toxics(reader, writer, toxics, disband_sender, toxic_state_holder)
        }
    }

    fn forward_direct(
        &mut self,
        mut reader: Read,
        mut writer: Write,
        disband_sender: oneshot::Sender<Ends>,
    ) -> JoinHandle<()> {
        let mut stop = self.stop.clone();
        tokio::spawn(async move {
            if !stop.stop_received() {
                let forward_res = forward(&mut reader, &mut writer, &mut stop).await;
                if let Err(err) = forward_res {
                    // TODO: maybe log this error in case it's a specific I/O error.
                }
            }
            let _ = disband_sender.send((reader, writer));
        })
    }

    fn setup_toxics(
        &mut self,
        reader: Read,
        writer: Write,
        toxics: Vec<Toxic>,
        disband_sender: oneshot::Sender<Ends>,
        toxic_state_holder: Option<Arc<ToxicStateHolder>>,
    ) -> JoinHandle<()> {
        let mut stop = self.stop.clone();
        let (left_end_tx, left_end_rx) = futures_mpsc::channel::<Bytes>(1);
        let (right_end_tx, right_end_rx) = futures_mpsc::channel::<Bytes>(1);

        let rand_gen = if let Some(seed) = self.config.rand_seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_entropy()
        };
        let mut toxic_runners: Vec<ToxicRunner> = toxics
            .into_iter()
            .zip(rand_gen.sample_iter(Standard))
            .map(ToxicRunner::new)
            .collect();

        let (close_read_join, close_write_join): (
            JoinHandle<io::Result<Read>>,
            JoinHandle<io::Result<Write>>,
        ) = self.connect_pipe_ends(
            reader,
            writer,
            &mut toxic_runners,
            &mut stop,
            left_end_tx,
            right_end_rx,
        );

        let join_handle =
            self.prepare_link_join_handle(close_read_join, close_write_join, disband_sender);

        let mut prev_pipe_read_rx = left_end_rx;

        for runner in toxic_runners {
            prev_pipe_read_rx = self.start_toxic_runner(
                runner,
                &mut stop,
                prev_pipe_read_rx,
                toxic_state_holder.clone(),
            );
        }

        tokio::spawn(async move { prev_pipe_read_rx.map(Ok).forward(right_end_tx).await });

        join_handle
    }

    fn start_toxic_runner(
        &self,
        mut runner: ToxicRunner,
        stop: &mut Stop,
        prev_pipe_read_rx: futures_mpsc::Receiver<Bytes>,
        toxic_state_holder: Option<Arc<ToxicStateHolder>>,
    ) -> futures_mpsc::Receiver<Bytes> {
        let toxic_name = runner.toxic_name();
        let toxic_state =
            toxic_state_holder.and_then(|holder| holder.get_state_for_toxic(toxic_name));
        let stop = stop.clone();
        let rand_seed = self.config.rand_seed;
        // Get the desired channel buffer capacity for the toxic (in number of chunks)
        // This is 1024 for the Latency toxic and 1 for others, similar
        // to the original Toxiproxy implementation.
        let (pipe_tx, pipe_rx) =
            futures_mpsc::channel::<Bytes>(runner.toxic_kind().chunk_buffer_capacity());
        let mut stop = stop.clone();
        tokio::spawn(async move {
            let maybe_res = tokio::select! {
                res = runner.run(prev_pipe_read_rx, pipe_tx, toxic_state, rand_seed) => Some(res),
                _ = stop.recv() => None,
            };
            if let Some(res) = maybe_res {
                if let Err(_err) = res {
                    // TODO: log this error from the toxic
                }
            }
        });
        pipe_rx
    }

    fn connect_pipe_ends(
        &self,
        reader: Read,
        writer: Write,
        mut toxic_runners: &mut [ToxicRunner],
        stop: &mut Stop,
        left_end_tx: futures_mpsc::Sender<Bytes>,
        right_end_rx: futures_mpsc::Receiver<Bytes>,
    ) -> (JoinHandle<io::Result<Read>>, JoinHandle<io::Result<Write>>) {
        let (mut stop_read, read_stopper) = stop.fork();
        let (mut stop_write, write_stopper) = stop.fork();

        let (override_stop_toxics, toxic_override_stopper) = stop.fork();
        let toxic_override_stopper_clone = toxic_override_stopper.clone();

        let wait_for_manual_close: Option<Close> =
            self.prepare_manual_close_signals(&mut toxic_runners, override_stop_toxics);

        let wait_for_manual_close_write = wait_for_manual_close.clone();
        let close_read_join = tokio::spawn(async move {
            pin!(left_end_tx);
            let res = forward_read(reader, left_end_tx, &mut stop_read).await;
            // Speed up closing the underlying connection by closing the other channel,
            // unless we should wait for a toxic to yield explicitly.
            if wait_for_manual_close.is_none() || stop_read.stop_received() {
                write_stopper.stop();
            } else if let Some(close) = wait_for_manual_close {
                toxic_override_stopper.stop();
                let _ = close.recv().await;
                write_stopper.stop();
            }
            res
        });

        let close_write_join = tokio::spawn(async move {
            pin!(right_end_rx);
            let res = forward_write(right_end_rx, writer, &mut stop_write).await;
            // Speed up closing the underlying connection by closing the other channel,
            // unless we should wait for a toxic to yield explicitly.
            if wait_for_manual_close_write.is_none() || stop_write.stop_received() {
                read_stopper.stop();
            } else if let Some(close) = wait_for_manual_close_write {
                toxic_override_stopper_clone.stop();
                let _ = close.recv().await;
                read_stopper.stop();
            }
            res
        });
        (close_read_join, close_write_join)
    }

    fn prepare_manual_close_signals(
        &self,
        toxic_runners: &mut [ToxicRunner],
        override_stop_toxics: Stop,
    ) -> Option<Close> {
        let close_signals: Vec<Close> = toxic_runners
            .iter_mut()
            .filter_map(|runner| {
                if runner.is_active() && runner.toxic_kind().has_close_logic() {
                    let (close, closer) = Close::new();
                    runner.set_closer(closer);
                    runner.set_override_stop(override_stop_toxics.clone());
                    Some(close)
                } else {
                    None
                }
            })
            .collect();
        if !close_signals.is_empty() {
            let (close, closer) = Close::new();

            tokio::spawn(async move {
                for close in close_signals {
                    let _ = close.recv().await;
                }
                let _ = closer.close();
            });
            Some(close)
        } else {
            None
        }
    }

    fn prepare_link_join_handle(
        &mut self,
        close_read_join: JoinHandle<io::Result<Read>>,
        close_write_join: JoinHandle<io::Result<Write>>,
        disband_sender: oneshot::Sender<Ends>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let result: Result<(io::Result<Read>, io::Result<Write>), tokio::task::JoinError> =
                tokio::try_join!(close_read_join, close_write_join);
            match result {
                Ok((read_res, write_res)) => {
                    if let Ok(reader) = read_res {
                        if let Ok(writer) = write_res {
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
        })
    }

    /// Cuts all the streams, stops all the ToxicRunner tasks, returns the original
    /// stream and the sink at the two ends.
    pub(super) async fn disband(self) -> io::Result<(Read, Write)> {
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
    active: bool,
    toxic: Toxic,
    closer: Option<Closer>,
    override_stop: Option<Stop>,
}

impl ToxicRunner {
    pub fn new((toxic, threshold): (Toxic, f32)) -> Self {
        ToxicRunner {
            active: toxic.toxicity > threshold,
            toxic,
            closer: None,
            override_stop: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn toxic_name(&self) -> &str {
        &self.toxic.name
    }

    pub fn toxic_kind(&self) -> &ToxicKind {
        &self.toxic.kind
    }

    pub fn set_closer(&mut self, closer: Closer) {
        self.closer = Some(closer);
    }

    pub fn set_override_stop(&mut self, stop: Stop) {
        self.override_stop = Some(stop);
    }

    fn take_override_stop(&mut self) -> Stop {
        self.override_stop.take().expect(&format!(
            "State error: cannot run toxic {} without a override stop signal",
            self.toxic
        ))
    }

    pub async fn run(
        &mut self,
        input: impl Stream<Item = Bytes>,
        output: impl Sink<Bytes>,
        state: Option<Arc<AsyncMutex<ToxicState>>>,
        rand_seed: Option<u64>,
    ) -> io::Result<()> {
        pin!(input);
        pin!(output);
        let result = if self.active {
            match self.toxic.kind {
                ToxicKind::Noop => toxics::run_noop(input, output).await,
                ToxicKind::Latency { latency, jitter } => {
                    toxics::run_latency(input, output, latency, jitter, rand_seed).await
                }
                ToxicKind::Timeout { timeout } => toxics::run_timeout(input, output, timeout).await,
                ToxicKind::Bandwidth { rate } => toxics::run_bandwidth(input, output, rate).await,
                ToxicKind::SlowClose { delay } => {
                    let stop = self.take_override_stop();
                    toxics::run_slow_close(input, output, stop, delay).await
                }
                ToxicKind::Slicer {
                    average_size,
                    size_variation,
                    delay,
                } => {
                    toxics::run_slicer(
                        input,
                        output,
                        average_size,
                        size_variation,
                        delay,
                        rand_seed,
                    )
                    .await
                }
                ToxicKind::LimitData { bytes } => {
                    let stop = self.take_override_stop();
                    toxics::run_limit_data(input, output, stop, bytes, state).await
                }
            }
        } else {
            toxics::run_noop(input, output).await
        };
        if let Some(closer) = self.closer.take() {
            let _ = closer.close();
        }
        return result;
    }
}
