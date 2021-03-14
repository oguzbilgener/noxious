use crate::{signal::Stop, state::ToxicState};
use bytes::Bytes;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::convert::TryInto;
use std::{io, sync::Arc};
use tokio::pin;
use tokio::sync::Mutex as AsyncMutex;

/// Run the slicer toxic
pub(crate) async fn run_limit_data(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
    mut stop: Stop,
    bytes: u64,
    state: Option<Arc<AsyncMutex<ToxicState>>>,
) -> io::Result<()> {
    let state = state.expect("No toxic state provided to LimitData toxic.");
    pin!(input);
    pin!(output);
    let mut state = state.lock().await;
    let bytes: usize = bytes
        .try_into()
        .expect("Could not convert bytes limit from u64 to usize");

    let mut bytes_transmitted: usize = get_bytes_transmitted(&state);
    let mut result = io::Result::Ok(());

    while !stop.stop_received() {
        if bytes_transmitted < bytes {
            let maybe_chunk = tokio::select! {
                res = input.next() => res,
                _ = stop.recv() => None,
            };

            if let Some(mut chunk) = maybe_chunk {
                let remaining: usize = bytes - bytes_transmitted;
                if remaining > 0 {
                    chunk.truncate(remaining);
                    let to_send = chunk.len();

                    if output.send(chunk).await.is_ok() {
                        bytes_transmitted += to_send;
                    } else {
                        result = Err(io::Error::new(
                            io::ErrorKind::ConnectionReset,
                            "limit data channel closed",
                        ))
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    write_bytes_transmitted(&mut state, bytes_transmitted);
    result
}

fn get_bytes_transmitted(state: &ToxicState) -> usize {
    #[allow(irrefutable_let_patterns)]
    if let ToxicState::LimitData { bytes_transmitted } = state {
        *bytes_transmitted
    } else {
        panic!("Invalid ToxicState given to LimitData toxic: {:?}", state);
    }
}

fn write_bytes_transmitted(state: &mut ToxicState, value: usize) {
    #[allow(irrefutable_let_patterns)]
    if let ToxicState::LimitData { bytes_transmitted } = state {
        *bytes_transmitted = value;
    } else {
        panic!("Invalid ToxicState given to LimitData toxic: {:?}", state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{toxic::ToxicKind, toxics::test_utils::*};
    use futures::{SinkExt, StreamExt};
    use tokio_test::assert_ok;

    #[tokio::test]
    #[should_panic(expected = "No toxic state provided to LimitData toxic.")]
    async fn panics_without_state() {
        let (in_stream, _) = create_stream_sink();
        let (_, out_sink) = create_stream_sink();
        let (stop, _) = Stop::new();
        let _ = run_limit_data(in_stream, out_sink, stop, 0, None).await;
    }

    async fn test_limit_data(
        limit: u64,
        to_send: u64,
        prev_state: Option<Arc<AsyncMutex<ToxicState>>>,
        expect_output: bool,
    ) {
        let (in_stream, mut in_sink) = create_stream_sink();
        let (mut out_stream, out_sink) = create_stream_sink();
        let (stop, stopper) = Stop::new();
        let handle = tokio::spawn(run_limit_data(in_stream, out_sink, stop, limit, prev_state));

        let data = gen_random_bytes(to_send.try_into().unwrap());

        let mut expected = data.clone();
        expected.truncate(limit as usize);

        assert_ok!(in_sink.send(data).await);
        if to_send == 0 || limit == 0 {
            assert_eq!(None, out_stream.next().await);
        } else if expect_output {
            let output = out_stream.next().await.unwrap();
            assert_eq!(expected, output);
        }

        if to_send < limit {
            stopper.stop();
        }
        let res = handle.await.unwrap();
        assert_ok!(res);
    }

    fn make_state() -> Arc<AsyncMutex<ToxicState>> {
        Arc::new(AsyncMutex::new(
            ToxicState::for_toxic_kind(&ToxicKind::LimitData { bytes: 0 }).unwrap(),
        ))
    }

    #[tokio::test]
    async fn limit_0() {
        test_limit_data(0, 32, Some(make_state()), false).await;
    }

    #[tokio::test]
    async fn send_below_limit() {
        test_limit_data(10000, 500, Some(make_state()), true).await;
    }

    #[tokio::test]
    async fn send_above_limit() {
        test_limit_data(42, 500, Some(make_state()), false).await;
    }

    #[tokio::test]
    async fn send_state_above_limit() {
        let state = Arc::new(AsyncMutex::new(
            ToxicState::for_toxic_kind(&ToxicKind::LimitData { bytes: 99999 }).unwrap(),
        ));
        test_limit_data(42, 500, Some(state), false).await;
    }
}
