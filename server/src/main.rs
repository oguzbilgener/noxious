use chrono::{Timelike, Utc};
use std::default::Default;
use tokio::net::TcpListener;
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
    let host = "localhost";
    let port = "1234";
    // TODO: clean up errors
    let listener = TcpListener::bind(&format!("{}:{}", host, port))
        .await
        .expect("failed to bind");

    // TODO: parse the json file, deserialize all toxics, start the core server

    // TODO: harmonious shutdown handling
    noxious::run(Vec::new(), signal::ctrl_c())
        .await
        .expect("uh?");

    api::run_server(listener, signal::ctrl_c())
        .await
        .expect("failed?");
}
