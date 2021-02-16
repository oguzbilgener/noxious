use std::future::Future;
use tokio::{sync::broadcast, task::JoinHandle};

#[derive(Debug)]
pub(crate) struct Stop {
    stop: bool,
    receiver: broadcast::Receiver<()>,
    sender: broadcast::Sender<()>,
}

impl Stop {
    pub(crate) fn new() -> (Stop, Stopper) {
        let (sender, receiver) = broadcast::channel::<()>(1);
        let stopper = Stopper::new(sender.clone());
        let stop = Stop {
            stop: false,
            receiver,
            sender,
        };
        (stop, stopper)
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stop
    }

    pub(crate) async fn recv(&mut self) {
        if self.stop {
            return;
        }

        let _ = self.receiver.recv().await;

        self.stop = true;
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
            stop: self.stop,
            receiver: self.sender.subscribe(),
            sender: self.sender.clone(),
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

pub(crate) fn spawn_stoppable<T>(mut stop: Stop, task: T) -> JoinHandle<Option<T::Output>>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    tokio::spawn(async move {
        if stop.is_stopped() {
            return None;
        }
        tokio::select! {
            res = task => Some(res),
            _ = stop.recv() => None,
        }
    })
}
