use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

use crate::signal::Stop;
use crate::link::Link;

#[derive(Debug, Clone, PartialEq)]
pub struct ProxyConfig {
    /// An arbitrary name
    name: String,
    /// The host name and the port the proxy listens on, like 127.0.0.1:5431
    listen: String,
    /// The host name and the port the proxy connects to, like 127.0.0:5432
    upstream: String,
    /// Buffer size for this proxy, in bytes
    buffer_size: usize
}

/// An upstream or downstream connection


pub(crate) async fn run_proxy(config: ProxyConfig, mut stop: Stop) -> io::Result<()> {
    let listener = TcpListener::bind(config.listen).await?;
    let buffer_size = config.buffer_size;

    while !stop.is_shutdown() {
        let maybe_connection = tokio::select! {
            res = listener.accept() => Ok::<Option<(TcpStream, SocketAddr)>, io::Error>(Some(res?)),
            _ = stop.recv() => {
                Ok(None)
            },
        }?;

        if let Some((stream, addr)) = maybe_connection {
            let stop = stop.clone();
            tokio::spawn(async move {
                let mut link = Link::new(stream, addr, buffer_size);
                link.handle(stop).await;
            });
        } else {
            break;
        }
    }
    Ok(())
}

