use std::default::Default;
use std::net::SocketAddr;
use tokio::signal;

mod api;

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

    // TODO: parse the json file, deserialize all toxics, start the core server

    // TODO: harmonious shutdown handling
    // noxious::run(Vec::new(), signal::ctrl_c())
    //     .await
    //     .expect("uh?");

    api::serve(SocketAddr::new([127, 0, 0, 1].into(), 1234), signal::ctrl_c())
        .await;
}
