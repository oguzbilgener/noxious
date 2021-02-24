use super::Sender;
use crate::util;
use std::{fmt::Debug, future::Future, net::SocketAddr};
use tracing::{info, instrument};
use warp::{Filter, Rejection, Reply};

// rest api
mod filters;
mod handlers;

fn make_filters(sender: Sender) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    use filters::*;

    disallow_browsers()
        .or(reset(sender.clone())
            .or(populate(sender.clone()))
            .or(version()))
        .or(get_proxies(sender.clone())
            .or(create_proxy(sender.clone()).or(get_proxy(sender.clone()))))
        .or(update_proxy(sender.clone()).or(delete_proxy(sender.clone())))
        .or(get_toxics(sender.clone())
            .or(create_toxics(sender.clone()))
            .or(update_toxic(sender.clone())))
        .or(get_toxic(sender.clone()).or(delete_toxic(sender.clone())))
}

pub async fn serve(addr: SocketAddr, request_sender: Sender, shutdown: impl Future) {
    let version = util::get_version();
    info!(
        addr = ?addr,
        version = ?version,
        "API HTTP server starting"
    );

    let api = make_filters(request_sender);
    let routes = api.with(warp::log("noxious"));
    tokio::select! {
        _ = warp::serve(routes).run(addr) => {},
        _ = shutdown => {},
    };
}
