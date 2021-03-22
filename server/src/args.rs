use clap::Clap;
use std::net::IpAddr;

/// A Rust port of Toxiproxy server
#[derive(Clap, Debug)]
pub struct Args {
    /// The host to listen on for the API server
    #[clap(short, long, default_value = "127.0.0.1")]
    pub host: String,
    /// The port to listen on for the API server
    #[clap(short, long, default_value = "8474")]
    pub port: String,
    /// json file containing proxies to create on startup
    #[clap(short, long)]
    pub config: Option<String>,
    /// Seed for randomizing toxics with
    #[clap(long)]
    pub seed: Option<u64>,
}

impl Args {
    pub fn get_ip_addr(&self) -> IpAddr {
        self.host.parse().expect("Invalid host address")
    }
    pub fn get_port_number(&self) -> u16 {
        self.port.parse().expect("Invalid port number")
    }
}
