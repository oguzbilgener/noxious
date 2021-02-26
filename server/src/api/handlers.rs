use crate::store::Store;
use crate::util;
use noxious::ProxyConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::convert::Infallible;
use tracing::warn;
use warp::http::StatusCode;
use warp::{reply::Response, Reply};

pub async fn reset_state(store: Store) -> Result<impl Reply, Infallible> {
    if let Err(err) = store.reset_state().await {
        warn!(
            err = ?err,
            "ResetState: Failed to write headers to client"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Re-populate the toxics from the initial config, return a map of proxies with toxics
pub async fn populate(configs: Vec<ProxyConfig>, store: Store) -> Result<impl Reply, Infallible> {
    match store.populate(configs).await {
        Ok(proxy_map) => Ok(warp::reply::json(&proxy_map).into_response()),
        Err(err) => Ok(responses::response_with_error(
            err,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

/// Get a key-value map of all proxies and their toxics in the system
pub async fn get_proxies(store: Store) -> Result<impl Reply, Infallible> {
    let result = store.get_proxies().await.and_then(|pairs| {
        let maybe_map: Result<serde_json::Map<String, JsonValue>, anyhow::Error> = pairs
            .into_iter()
            .try_fold(serde_json::Map::new(), |mut acc, pair| {
                let key = pair.proxy.name.clone();
                let value = serde_json::to_value(pair)?;
                acc.insert(key, value);
                Ok(acc)
            });
        maybe_map
    });
    match result {
        Ok(proxy_map) => Ok(warp::reply::json(&proxy_map).into_response()),
        Err(err) => Ok(responses::response_with_error(
            err,
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn create_proxy(proxy: ProxyConfig, store: Store) -> Result<impl Reply, Infallible> {
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

    pub fn respond_with_data(data: impl Serialize) -> Result<impl Reply, Infallible> {
        Ok(with_status(json_reply(&data), StatusCode::OK))
    }

    pub fn response_with_error(data: anyhow::Error, status: StatusCode) -> Response {
        let body = json!({ "message": data.to_string() });
        with_status(json_reply(&body), status).into_response()
    }
}
