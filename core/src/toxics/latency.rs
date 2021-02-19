use bytes::Bytes;
use futures::StreamExt;
use futures::{Sink, Stream};
use rand::distributions::Uniform;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::io;
use std::time::Duration;

/// Run the latency toxic
///
/// This implementation has a slightly different behavior from Shopify's toxiproxy
/// when it comes to randomizing jitter. Toxiproxy uses the global random number
/// generator from Go's rand package. There is no equivalent for this in Rust
/// and we don't want to use another Mutex to share a thread-local random generator
/// that's seeded once at startup, so we're seeding a new random generator for every
/// latency toxic, with the same seed startup argument, if available.
/// This would still allow determinism when you need it.
pub async fn run_latency(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    latency: u64,
    jitter: u64,
) -> io::Result<()> {
    let range = Uniform::from(0..(jitter * 2));
    let rng = StdRng::from_entropy();
    let jitter_stream = futures::stream::iter(rng.sample_iter(&range));

    // let mut rng = if let Some(seed) = rand_seed {
    //     StdRng::seed_from_u64(seed)
    // } else {
    //     StdRng::from_entropy()
    // };
    let _ = input
        .zip(jitter_stream)
        .then(|(chunk, add)| async move {
            let mut delay = latency;
            println!("delay {} add {} jitter {}", delay, add, jitter);
            if jitter > 0 {
                delay = latency + add - jitter;
            }
            println!("Delaying by {}", delay);
            tokio::time::sleep(Duration::from_millis(delay)).await;
            chunk
        })
        .map(Ok)
        .forward(output)
        .await;
    Ok(())
}
