use bytes::Bytes;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::convert::TryInto;
use std::io;
use tokio::pin;
use tokio::time::sleep;
use tokio::time::Duration;

/// Run the slicer toxic
pub async fn run_slicer(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    average_size: u64,
    size_variation: u64,
    delay: u64, // microseconds
    rand_seed: Option<u64>,
) -> io::Result<()> {
    pin!(input);
    pin!(output);

    while let Some(chunk) = input.next().await {
        let mut slice_iter = SliceIter::new(chunk, average_size, size_variation, rand_seed);
        while let Some(slice) = slice_iter.next() {
            sleep(Duration::from_micros(delay)).await;
            if let Err(_) = output.send(slice).await {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "Write channel closed",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
enum SliceIterKind {
    ConstantSized,
    VariableSized {
        size_variation: usize,
        rand_gen: StdRng,
    },
}

#[derive(Debug)]
struct SliceIter {
    data: Option<Bytes>,
    average_size: usize,
    kind: SliceIterKind,
}

impl SliceIter {
    fn new(
        data: Bytes,
        average_size: u64,
        size_variation: u64,
        rand_seed: Option<u64>,
    ) -> SliceIter {
        let kind = if size_variation > 0 {
            let rand_gen = if let Some(seed) = rand_seed {
                StdRng::seed_from_u64(seed)
            } else {
                StdRng::from_entropy()
            };
            SliceIterKind::VariableSized {
                size_variation: size_variation
                    .try_into()
                    .expect("Could not convert size_variation from u64 to usize"),
                rand_gen,
            }
        } else {
            SliceIterKind::ConstantSized
        };
        SliceIter {
            data: Some(data),
            average_size: average_size
                .try_into()
                .expect("Could not convert average_size from u64 to usize"),
            kind,
        }
    }

    fn slice_data(&mut self, position: usize) -> Option<Bytes> {
        if let Some(mut data) = self.data.take() {
            if data.len() > position {
                let slice = data.split_to(position);
                self.data = Some(data);
                Some(slice)
            } else if data.len() > 0 {
                Some(data)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Iterator for SliceIter {
    type Item = Bytes;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(_) = &self.data {
            match &mut self.kind {
                SliceIterKind::ConstantSized => self.slice_data(self.average_size),
                SliceIterKind::VariableSized {
                    size_variation,
                    rand_gen,
                } => {
                    let variation = *size_variation;
                    let size =
                        self.average_size + 2 * rand_gen.gen_range(1..=variation) - variation;
                    self.slice_data(size)
                }
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toxics::test_utils::*;

    #[tokio::test]
    async fn passthrough_once() {
        passthrough_test(
            |stream, sink| async move { run_slicer(stream, sink, 50, 0, 0, None).await },
        )
        .await;
    }

    #[tokio::test]
    async fn random_seed_passthrough_once() {
        passthrough_test(|stream, sink| async move {
            run_slicer(stream, sink, 50, 0, 0, Some(42)).await
        })
        .await;
    }

    #[tokio::test]
    async fn random_seed_variation_passthrough_once() {
        passthrough_test(|stream, sink| async move {
            run_slicer(stream, sink, 50, 8, 0, Some(42)).await
        })
        .await;
    }

    #[tokio::test]
    async fn variation_passthrough_once() {
        passthrough_test(
            |stream, sink| async move { run_slicer(stream, sink, 50, 8, 0, None).await },
        )
        .await;
    }
}
