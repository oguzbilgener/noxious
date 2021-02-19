use std::sync::Arc;
use tokio::sync::Mutex;

use futures::{FutureExt, Stream};
use rand::prelude::StdRng;

#[derive(Debug, Clone)]
pub struct RandomStream {
    seeded_rng: Option<Arc<Mutex<StdRng>>>,
}

impl RandomStream {
    pub fn new(seed: Option<u64>) {
        todo!()
    }
}

