use std::future::Future;
use std::io;
use tokio::net::TcpListener;

// rest api

pub async fn run_server(listener: TcpListener, shutdown: impl Future) -> io::Result<()> {
    Ok(())
}
