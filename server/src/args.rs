use clap::Parser;
use std::net::{IpAddr, Ipv4Addr};

/// A Rust port of Toxiproxy server
#[derive(Parser, Debug)]
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
        if self.host == "localhost" {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            self.host.parse().expect("Invalid host address")
        }
    }
    pub fn get_port_number(&self) -> u16 {
        self.port.parse().expect("Invalid port number")
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn parses_ip_addr() {
        let input = Args {
            host: "127.0.0.1".to_owned(),
            port: "5555".to_owned(),
            config: None,
            seed: None,
        };
        let addr = input.get_ip_addr();
        let expected = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(expected, addr);
    }

    #[test]
    fn parses_localhost() {
        let input = Args {
            host: "localhost".to_owned(),
            port: "5555".to_owned(),
            config: None,
            seed: None,
        };
        let addr = input.get_ip_addr();
        let expected = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        assert_eq!(expected, addr);
    }

    #[test]
    fn parses_port_num() {
        let input = Args {
            host: "127.0.0.1".to_owned(),
            port: "5555".to_owned(),
            config: None,
            seed: None,
        };
        let port = input.get_port_number();
        let expected = 5555u16;
        assert_eq!(expected, port);
    }

    #[test]
    #[should_panic]
    fn panics_on_invalid_ip() {
        let input = Args {
            host: "127.0.0.1.2".to_owned(),
            port: "5555".to_owned(),
            config: None,
            seed: None,
        };
        let _addr = input.get_ip_addr();
    }

    #[test]
    #[should_panic]
    fn panics_on_invalid_port() {
        let input = Args {
            host: "127.0.0.1".to_owned(),
            port: "555511111".to_owned(),
            config: None,
            seed: None,
        };
        let _port = input.get_port_number();
    }
}
