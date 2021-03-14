use thiserror::Error;
use tokio::sync::{broadcast, watch};
use tracing::instrument;

/// The receiver for the stop signal, which can be used to indicate that
/// a part, or all of the system is required to shut down.
/// This stop handle can be cloned to pass the signal to multiple async tasks,
/// and it can be forked to let child tasks have their own stop logic in addition
/// to the parent system  stop logic.
#[derive(Debug)]
pub struct Stop {
    stopped: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl Stop {
    /// Create a new Stop and Stopper
    pub fn new() -> (Stop, Stopper) {
        let (sender, receiver) = broadcast::channel::<()>(1);
        let stopper = Stopper::new(sender.clone());
        let stop = Stop {
            stopped: false,
            receiver,
            sender,
        };
        (stop, stopper)
    }

    /// Check if this particular instance of Stop has received a stop signal.
    /// Note: Only use this in conjunction with `recv`, because if this instance
    /// of Stop does not receive the signal, this will return false.
    pub fn stop_received(&self) -> bool {
        self.stopped
    }

    /// Wait for the stop signal to be received
    pub async fn recv(&mut self) {
        if self.stopped {
            return;
        }

        let _ = self.receiver.recv().await;

        self.stopped = true;
    }

    /// Creates a sub-signal that has its own stopper but propagates the stop signal from the original
    pub fn fork(&self) -> (Stop, Stopper) {
        let (forked_stop, forked_stopper) = Stop::new();
        let forked_sender = forked_stop.sender.clone();
        let mut original_receiver = self.sender.subscribe();
        tokio::spawn(async move {
            while original_receiver.recv().await.is_ok() {
                if forked_sender.send(()).is_err() {
                    // Channel closed, we can no longer forward signal from original to fork
                    break;
                }
            }
            drop(forked_sender);
        });
        (forked_stop, forked_stopper)
    }

    /// Creates a new stopper for this Stop signal that can be used to stop this Stop and all
    /// its forked descendants
    pub fn get_stopper(&self) -> Stopper {
        Stopper::new(self.sender.clone())
    }
}

impl Clone for Stop {
    fn clone(&self) -> Self {
        Self {
            stopped: self.stopped,
            receiver: self.sender.subscribe(),
            sender: self.sender.clone(),
        }
    }
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.stop_received() {
            write!(f, "stopped")
        } else {
            write!(f, "NOT stopped")
        }
    }
}

/// A handle that can send a stop signal once to all subscribers
#[derive(Debug, Clone)]
pub struct Stopper {
    sender: broadcast::Sender<()>,
}

impl Stopper {
    pub(crate) fn new(sender: broadcast::Sender<()>) -> Self {
        Self { sender }
    }

    /// Sends the stop signal
    #[instrument(level = "trace", skip(self))]
    pub fn stop(self) {
        let _ = self.sender.send(());
    }
}
/// A receiver for the close signal, which is used to indicate that a resource
/// is ready to close
#[derive(Debug, Clone)]
pub struct Close {
    receiver: watch::Receiver<Option<()>>,
}

/// The sender for the close signal, to indicate that the owner of this closer
/// is ready to close
#[derive(Debug)]
pub struct Closer {
    sender: watch::Sender<Option<()>>,
}

/// The listen channel closed before the close signal was received
#[derive(Error, Copy, Clone, Debug)]
#[error("Close channel closed")]
pub struct CloseError;

/// Could not snd the close signal, listener for close dropped
#[derive(Error, Copy, Clone, Debug)]
#[error("Could not close, already closed?")]
pub struct CloserError;

impl Close {
    /// Create a new Close and Closer
    pub fn new() -> (Close, Closer) {
        let (sender, receiver) = watch::channel(None);
        let close = Close { receiver };
        let closer = Closer { sender };
        (close, closer)
    }

    /// Wait for the close signal
    pub async fn recv(mut self) -> Result<(), CloseError> {
        self.receiver.changed().await.map_err(|_| CloseError)
    }
}

impl Closer {
    /// Send the close signal and consume this closer
    pub fn close(self) -> Result<(), CloseError> {
        self.sender.send(Some(())).map_err(|_| CloseError)
    }
}
