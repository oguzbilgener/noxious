use crate::state::ToxicState;
use bytes::Bytes;
use futures::{Sink, Stream};
use futures::{SinkExt, StreamExt};
use std::convert::TryInto;
use std::{io, sync::Arc};
use tokio::pin;
use tokio::sync::Mutex as AsyncMutex;

/// Run the slicer toxic
pub async fn run_limit_data(
    input: impl Stream<Item = Bytes>,
    output: impl Sink<Bytes>,
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

    while bytes_transmitted < bytes {
        if let Some(mut chunk) = input.next().await {
            let remaining: usize = bytes - bytes_transmitted;
            if remaining > 0 {
                chunk.truncate(remaining);
                let to_send = chunk.len();

                if let Ok(_) = output.send(chunk).await {
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
    }

    write_bytes_transmitted(&mut state, bytes_transmitted);
    println!("limit toxic closing {}", result.is_ok());
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
