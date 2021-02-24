use std::{future::Future, net::SocketAddr};
use warp::{Filter, Rejection, Reply};

// rest api
mod filters;
mod handlers;

fn make_filters() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    use filters::*;

    disallow_browsers()
        .or(reset().or(populate()).or(version()))
        .or(get_proxies().or(create_proxy().or(get_proxy())))
        .or(update_proxy().or(delete_proxy()))
        .or(get_toxics().or(create_toxics()))
        .or(get_toxic().or(delete_toxic()))
}

pub async fn serve(addr: SocketAddr, shutdown: impl Future) {
    let api = make_filters();
    let routes = api.with(warp::log("noxious"));
    tokio::select! {
        _ = warp::serve(routes).run(addr) => {},
        _ = shutdown => {},
    };
}
