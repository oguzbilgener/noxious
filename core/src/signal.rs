use tokio::sync::{broadcast, oneshot};

#[derive(Debug)]
pub(crate) struct Stop {
    shutdown: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl Stop {
    pub(crate) fn new(sender: broadcast::Sender<()>) -> Stop {
        Stop {
            shutdown: false,
            receiver: sender.subscribe(),
            sender,
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub(crate) async fn recv(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.receiver.recv().await;

        self.shutdown = true;
    }
}

impl Clone for Stop {
    fn clone(&self) -> Self {
        Self {
            shutdown: self.shutdown,
            receiver: self.sender.subscribe(),
            sender: self.sender.clone(),
        }
    }
}
