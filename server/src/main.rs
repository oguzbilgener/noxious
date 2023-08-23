use crate::args::Args;
use clap::Parser;
use noxious::{proxy::ProxyRunner, signal::Stop, socket::TcpListener};
use std::net::SocketAddr;
use tokio::signal;
use tracing::{debug, info};

use crate::{file::populate_initial_proxy_configs, store::Store};

mod api;
mod args;
mod error;
mod file;
mod store;
mod util;

#[tokio::main]
async fn main() {
    util::init_tracing();

    let args: Args = Args::parse();

    let (stop, stopper) = Stop::new();

    let store = Store::new(stop.clone(), args.seed);

    if let Some(config_file_path) = &args.config {
        populate_initial_proxy_configs::<TcpListener, ProxyRunner>(config_file_path, store.clone());
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
