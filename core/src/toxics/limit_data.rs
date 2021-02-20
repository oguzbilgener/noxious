use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use rand::distributions::Uniform;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::io;
use std::time::Duration;

/// Run the slicer toxic
pub async fn run_limit_data(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    bytes: u64
) -> io::Result<()> {


    Ok(())
}