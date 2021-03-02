use noxious::signal::Stop;
use std::net::SocketAddr;
use std::{default::Default, net::IpAddr};
use tokio::signal;
use tracing::{debug, error, info, instrument};

use crate::{file::populate_initial_proxy_configs, store::Store};

mod api;
mod error;
mod file;
mod store;
mod util;

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

impl Args {
    fn get_ip_addr(&self) -> IpAddr {
        self.host.parse().expect("Invalid host address")
    }
    fn get_port_number(&self) -> u16 {
        self.port.parse().expect("Invalid port number")
    }
}

#[tokio::main]
async fn main() {
    util::init_tracing();

    // TODO: parse the command line args

    let mut args = Args::default();
    args.config_file_path = Some("toxiproxy.json".to_owned());

    let (stop, stopper) = Stop::new();

    let store = Store::new(stop.clone());

    if let Some(config_file_path) = &args.config_file_path {
        populate_initial_proxy_configs(config_file_path, store.clone());
    } else {
        debug!("No config file path provided");
    }

    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        info!("Shutting down");
        stopper.stop();
    });

    api::serve(
        SocketAddr::new(args.get_ip_addr(), args.get_port_number()),
        store,
        stop,
    )
    .await;
}
