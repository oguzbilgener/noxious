use bytes::Bytes;
use futures::{stream, Sink, Stream, StreamExt};
use rand::distributions::Uniform;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::io;
use tokio::time::Duration;

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
    rand_seed: Option<u64>,
) -> io::Result<()> {
    if jitter == 0 {
        let _ = input
            .then(|chunk| async move {
                tokio::time::sleep(Duration::from_millis(latency)).await;
                chunk
            })
            .map(Ok)
            .forward(output)
            .await;
    } else {
        let range = Uniform::from(0..(jitter * 2));
        let rand_gen = if let Some(seed) = rand_seed {
            StdRng::seed_from_u64(seed)
        } else {
            StdRng::from_entropy()
        };
        let jitter_stream = stream::iter(rand_gen.sample_iter(&range));
        let _ = input
            .zip(jitter_stream)
            .then(|(chunk, add)| async move {
                let delay = latency + add - jitter;
                tokio::time::sleep(Duration::from_millis(delay)).await;
                chunk
            })
            .map(Ok)
            .forward(output)
            .await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::toxics::test_utils::*;
    use futures::{SinkExt, StreamExt};
    use tokio::time::{pause, resume, Instant};
    use tokio_test::assert_ok;

    #[tokio::test]
    async fn no_jitter_passthrough_once() {
        passthrough_test(|stream, sink| async move { run_latency(stream, sink, 2, 0, None).await })
            .await;
    }

    #[tokio::test]
    async fn passthrough_once() {
        passthrough_test(|stream, sink| async move { run_latency(stream, sink, 2, 2, None).await })
            .await;
    }

    #[tokio::test]
    async fn random_seed_passthrough_once() {
        passthrough_test(
            |stream, sink| async move { run_latency(stream, sink, 5, 2, Some(42)).await },
        )
        .await;
    }

    #[tokio::test]
    async fn drop_out_channel_first_with_latency() {
        drop_out_channel_first_test(|stream, sink| async move {
            run_latency(stream, sink, 2, 1, None).await
        })
        .await;
    }

    async fn test_latency(latency: u64, jitter: u64, seed: u64) {
        let (in_stream, mut in_sink) = create_stream_sink();
        let (mut out_stream, out_sink) = create_stream_sink();
        let data = gen_random_bytes(32);
        let expected = Some(data.clone());
        let handle = tokio::spawn(async move {
            run_latency(in_stream, out_sink, latency, jitter, Some(seed)).await
        });

        assert_ok!(in_sink.send(data).await);
        drop(in_sink);
        assert_ok!(handle.await.unwrap());
        assert_eq!(expected, out_stream.next().await);
    }

    #[tokio::test]
    async fn test_latency_10() {
        let latency = 10u64;
        let beginning = Instant::now();
        pause();
        test_latency(latency, 0, 0).await;
        let duration = Instant::now().duration_since(beginning);
        assert!(duration.as_millis() > latency as u128);
        resume();
    }

    #[tokio::test]
    async fn test_latency_with_jitter() {
        let latency = 1000u64;
        let jitter = 5u64;
        let beginning = Instant::now();
        pause();
        test_latency(latency, jitter, 1).await;
        let duration = Instant::now().duration_since(beginning);
        assert!(duration.as_millis() > latency as u128);
        resume();
    }
}
