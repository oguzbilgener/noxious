use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpSocket, TcpStream};

use crate::link::Link;
use crate::signal::Stop;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProxyConfig {
    /// An arbitrary name
    pub name: String,
    /// The host name and the port the proxy listens on, like 127.0.0.1:5431
    pub listen: String,
    /// The host name and the port the proxy connects to, like 127.0.0:5432
    pub upstream: String,
}

/// An upstream or downstream connection

pub(crate) async fn run_proxy(config: ProxyConfig, mut stop: Stop) -> io::Result<()> {
    let listener = TcpListener::bind(&config.listen).await?;

    while !stop.is_shutdown() {
        let maybe_connection = tokio::select! {
            res = listener.accept() => Ok::<Option<(TcpStream, SocketAddr)>, io::Error>(Some(res?)),
            _ = stop.recv() => {
                Ok(None)
            },
        }?;

        if let Some((client_stream, addr)) = maybe_connection {
            let upstream = TcpStream::connect(&config.upstream).await?;

            let (client_read, client_write) = client_stream.into_split();
            let (upstream_read, upstream_write) = upstream.into_split();

            let config = config.clone();
            let config_clone = config.clone();

            let stop = stop.clone();
            let stop_clone = stop.clone();

            // TODO: when there is an update in the list of toxics, drop the current link and
            // start a new one?

            tokio::spawn(async move {
                Link::new(client_read, upstream_write, addr, config)
                    .handle(stop)
                    .await
            });
            tokio::spawn(async move {
                Link::new(upstream_read, client_write, addr, config_clone)
                    .handle(stop_clone)
                    .await
            });
        } else {
            break;
        }
    }
    Ok(())
}
