use bmrng::{channel, RequestSender};
use noxious::error::NotFoundError;
use noxious::{ToxicEvent, ToxicEventKind};
use std::default::Default;
use std::net::SocketAddr;
use tokio::signal;
use crate::store::{Store, ProxyEvent, ProxyEventResult};
use tracing::{debug, instrument};

mod api;
mod util;
mod store;

/// The command line arguments for the server
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

    let (sender, receiver) = bmrng::channel::<ProxyEvent, ProxyEventResult>(16);

    let store = Store::new(sender);

    // TODO: parse the json file, deserialize all toxics, start proxy tasks
    store.populate();



    // TODO: harmonious shutdown handling
    // noxious::run(Vec::new(), signal::ctrl_c())
    //     .await
    //     .expect("uh?");

    api::serve(
        SocketAddr::new([127, 0, 0, 1].into(), 8474),
        store,
        signal::ctrl_c(),
    )
    .await;
}
