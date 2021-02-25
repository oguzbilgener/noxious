use std::convert::Infallible;
use warp::http::StatusCode;
// use serde_derive::{Deserialize, Serialize};
use crate::store::Store;
use crate::util;
use bmrng::RequestSender;
use noxious::error::NotFoundError;
use noxious::{ToxicEvent, ToxicEventKind};
use serde_json::{json, Value as JsonValue};
use tracing::warn;
use warp::{Filter, Rejection, Reply};

// #[derive(Serialize)]
// struct ErrorMessage {
//     code: u16,
//     message: String,
// }

pub async fn reset_state(store: Store) -> Result<impl Reply, Infallible> {
    if let Err(err) = store.reset_state() {
        warn!(
            err = ?err,
            "ResetState: Failed to write headers to client"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn populate(store: Store) -> Result<impl Reply, Infallible> {
    // TODO: return list of serializable toxics
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_proxies(store: Store) -> Result<impl Reply, Infallible> {
    // TODO: return a list of proxies
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_proxy(store: Store) -> Result<impl Reply, Infallible> {
    // TODO: return the proxy
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    // TODO: return the proxy or 404
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_toxic(name: String, store: Store) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_toxics(name: String, store: Store) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    // TODO: implement
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_version() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        util::get_version(),
        StatusCode::OK,
    ))
}

mod responses {
    use serde::Serialize;
    use std::error::Error;
    use warp::reply::{json as json_reply, with_status};

    use super::*;

    fn respond_with_data(data: impl Serialize) -> Result<impl Reply, Infallible> {
        let body = json!({ "data": data });
        Ok(with_status(json_reply(&body), StatusCode::OK))
    }

    fn respond_with_error(data: impl Error, status: StatusCode) -> Result<impl Reply, Infallible> {
        let body = json!({ "message": data.to_string() });
        Ok(with_status(json_reply(&body), status))
    }
}
