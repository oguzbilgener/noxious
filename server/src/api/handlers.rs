use crate::{
    error::StoreError,
    store::{SerializableToxic, Store},
    util,
};
use noxious::proxy::ProxyConfig;
use responses::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::convert::Infallible;
use std::future::Future;
use tracing::warn;
use warp::http::StatusCode;
use warp::{reply::Response, Reply};

/// Remove all toxics from all proxies
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
    wrap_store_result(async move { store.populate(configs).await }).await
}

/// Get a key-value map of all proxies and their toxics in the system
pub async fn get_proxies(store: Store) -> Result<impl Reply, Infallible> {
    let result = store.get_proxies().await.and_then(|pairs| {
        let maybe_map: Result<serde_json::Map<String, JsonValue>, serde_json::Error> = pairs
            .into_iter()
            .try_fold(serde_json::Map::new(), |mut acc, pair| {
                let key = pair.proxy.name.clone();
                let value = serde_json::to_value(pair)?;
                acc.insert(key, value);
                Ok(acc)
            });
        maybe_map.map_err(|_| StoreError::Other)
    });
    use_store_result(result)
}

/// Create a proxy, return it if successful
pub async fn create_proxy(proxy: ProxyConfig, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.create_proxy(proxy).await }).await
}

pub async fn get_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_proxy(name).await }).await
}

pub async fn update_proxy(
    name: String,
    new_proxy: ProxyConfig,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.update_proxy(name, new_proxy).await }).await
}

pub async fn remove_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result_no_content(async move { store.remove_proxy(name).await }).await
}

pub async fn get_toxics(proxy_name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_toxics(proxy_name).await }).await
}

pub async fn create_toxic(
    proxy_name: String,
    toxic: SerializableToxic,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.create_toxic(proxy_name, toxic).await }).await
}

pub async fn get_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_toxic(proxy_name, toxic_name).await }).await
}

pub async fn update_toxic(
    proxy_name: String,
    toxic_name: String,
    new_toxic: SerializableToxic,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.update_toxic(proxy_name, toxic_name, new_toxic).await })
        .await
}

pub async fn remove_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result_no_content(async move { store.remove_toxic(proxy_name, toxic_name).await })
        .await
}

pub async fn get_version() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        util::get_version(),
        StatusCode::OK,
    ))
}

mod responses {
    use serde::Serialize;
    use warp::reply::{json as json_reply, with_status};

    use crate::error::StoreError;

    use super::*;

    pub async fn wrap_store_result(
        f: impl Future<Output = Result<impl Serialize, StoreError>>,
    ) -> Result<impl Reply, Infallible> {
        use_store_result(f.await)
    }

    pub async fn wrap_store_result_no_content(
        f: impl Future<Output = Result<(), StoreError>>,
    ) -> Result<impl Reply, Infallible> {
        use_store_result_no_content(f.await)
    }

    pub fn use_store_result(
        result: Result<impl Serialize, StoreError>,
    ) -> Result<impl Reply, Infallible> {
        match result {
            Ok(data) => Ok(with_status(json_reply(&data), StatusCode::OK).into_response()),
            Err(err) => Ok(Into::<StatusCode>::into(err).into_response()),
        }
    }

    pub fn use_store_result_no_content(
        result: Result<(), StoreError>,
    ) -> Result<impl Reply, Infallible> {
        match result {
            Ok(_) => Ok(StatusCode::NO_CONTENT.into_response()),
            Err(err) => Ok(Into::<StatusCode>::into(err).into_response()),
        }
    }
}
