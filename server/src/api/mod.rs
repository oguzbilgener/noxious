use std::future::Future;
use std::io;
use tokio::net::TcpListener;

// rest api

pub async fn run_server(listener: TcpListener, shutdown: impl Future) -> io::Result<()> {
    Ok(())
}

// Endpoints to implement:
// - POST /reset
// - GET /proxies
// - POST /proxies
// - POST /populate
// - GET /proxies/{proxy}
// - POST /proxies/{proxy}
// - DELETE /proxies/{proxy}
// - GET /proxies/{proxy}/toxics
// - POST /proxies/{proxy}/toxics
// - GET /proxies/{proxy}/toxics/{toxic}
// - POST /proxies/{proxy}/toxics/{toxic}
// - DELETE /proxies/{proxy}/toxics/{toxic}