use crate::store::Store;
use crate::util;
use std::{fmt::Debug, future::Future, net::SocketAddr};
use tracing::{info, instrument};
use warp::{Filter, Rejection, Reply};

// rest api
mod filters;
mod handlers;

fn make_filters(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    use filters::*;

    disallow_browsers()
        .or(reset(store.clone())
            .or(populate(store.clone()))
            .or(version()))
        .or(get_proxies(store.clone()).or(create_proxy(store.clone()).or(get_proxy(store.clone()))))
        .or(update_proxy(store.clone()).or(remove_proxy(store.clone())))
        .or(get_toxics(store.clone())
            .or(create_toxics(store.clone()))
            .or(update_toxic(store.clone())))
        .or(get_toxic(store.clone()).or(remove_toxic(store.clone())))
}

pub async fn serve(addr: SocketAddr, store: Store, shutdown: impl Future) {
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
        _ = shutdown => {},
    };
}
