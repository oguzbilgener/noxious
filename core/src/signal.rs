use tokio::sync::broadcast;

#[derive(Debug)]
pub(crate) struct Stop {
    stopped: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl Stop {
    pub(crate) fn new() -> (Stop, Stopper) {
        let (sender, receiver) = broadcast::channel::<()>(1);
        let stopper = Stopper::new(sender.clone());
        let stop = Stop {
            stopped: false,
            receiver,
            sender,
        };
        (stop, stopper)
    }

    pub(crate) fn stop_received(&self) -> bool {
        self.stopped
    }

    pub(crate) async fn recv(&mut self) {
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

#[derive(Debug, Clone)]
pub struct Stopper {
    sender: broadcast::Sender<()>,
}

impl Stopper {
    pub fn new(sender: broadcast::Sender<()>) -> Self {
        Self { sender }
    }

    pub fn stop(self) {
        let _ = self.sender.send(());
    }
}
