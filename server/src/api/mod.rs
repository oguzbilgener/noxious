use crate::store::Store;
use crate::util;
use noxious::signal::Stop;
use std::{convert::Infallible, net::SocketAddr};
use tracing::{debug, info};
use warp::{Filter, Reply};

// rest api
mod filters;
mod handlers;

fn make_filters(store: Store) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    use filters::*;

    disallow_browsers()
        .or(reset(store.clone())
            .or(populate(store.clone()))
            .or(version()))
        .or(get_toxic(store.clone())
            .or(update_toxic(store.clone()))
            .or(create_toxic(store.clone())))
        .or(remove_toxic(store.clone()).or(get_toxics(store.clone())))
        .or(create_proxy(store.clone()).or(get_proxy(store.clone())))
        .or(update_proxy(store.clone())
            .or(remove_proxy(store.clone()))
            .or(get_proxies(store.clone())))
        .recover(handle_errors)
}

/// Serve the API server
/// Panics if the the provided SocketAddr is invalid or unavailable.
pub async fn serve(addr: SocketAddr, store: Store, mut stop: Stop) {
    let version = util::get_version();
    info!(
        addr = ?addr,
        version = ?version,
        "API HTTP server starting"
    );

    let api = make_filters(store);
    let routes = api.with(warp::log("noxious"));
    tokio::select! {
        _ = warp::serve(routes).run(addr) => {},
        _ = stop.recv() => {},
    };
    debug!("API HTTP server shutting down");
}
