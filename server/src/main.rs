use bmrng::{channel, RequestSender};
use noxious::error::NotFoundError;
use noxious::{ToxicEvent, ToxicEventKind};
use std::default::Default;
use std::net::SocketAddr;
use tokio::signal;
use tracing::{debug, instrument};

type Sender = RequestSender<ToxicEvent, Result<(), NotFoundError>>;

mod api;
mod util;

struct Args {
    /// The host to listen on for the API server
    host: &'static str,
    /// The port to listen on for the API server
    port: &'static str,
    /// json file containing proxies to create on startup
    config_file_path: Option<String>,
    /// Seed for randomizing toxics with
    seed: Option<u64>,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            host: "127.0.0.1",
            port: "8474",
            config_file_path: None,
            seed: None,
        }
    }
}

#[tokio::main]
async fn main() {
    util::init_tracing();

    let (sender, receiver) = bmrng::channel::<ToxicEvent, Result<(), NotFoundError>>(1);

    // TODO: parse the json file, deserialize all toxics, start the core server

    // TODO: harmonious shutdown handling
    // noxious::run(Vec::new(), signal::ctrl_c())
    //     .await
    //     .expect("uh?");

    api::serve(
        SocketAddr::new([127, 0, 0, 1].into(), 8474),
        sender,
        signal::ctrl_c(),
    )
    .await;
}
