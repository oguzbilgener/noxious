use thiserror::Error;
use tokio::sync::{broadcast, watch};

/// TODO
#[derive(Debug)]
pub struct Stop {
    stopped: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl Stop {
    /// TODO
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

    /// TODO
    pub fn stop_received(&self) -> bool {
        self.stopped
    }

    /// TODO
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
            while let Ok(_) = original_receiver.recv().await {
                if let Err(_) = forked_sender.send(()) {
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

/// TODO
#[derive(Debug, Clone)]
pub struct Stopper {
    sender: broadcast::Sender<()>,
}

impl Stopper {
    /// TODO
    pub fn new(sender: broadcast::Sender<()>) -> Self {
        Self { sender }
    }

    /// TODO: trace
    pub fn stop(self) {
        let _ = self.sender.send(());
    }
}
#[derive(Debug, Clone)]
pub struct Close {
    receiver: watch::Receiver<Option<()>>,
}

#[derive(Debug)]
pub struct Closer {
    sender: watch::Sender<Option<()>>,
}

#[derive(Error, Debug)]
#[error("Close channel closed")]
pub struct CloseError;

#[derive(Error, Debug)]
#[error("Could not close, already closed?")]
pub struct CloserError;

impl Close {
    pub fn new() -> (Close, Closer) {
        let (sender, receiver) = watch::channel(None);
        let close = Close { receiver };
        let closer = Closer { sender };
        (close, closer)
    }

    pub async fn recv(mut self) -> Result<(), CloseError> {
        self.receiver.changed().await.map_err(|_| CloseError)
    }
}

impl Closer {
    pub fn close(self) -> Result<(), CloseError> {
        self.sender.send(Some(())).map_err(|_| CloseError)
    }
}
