use crate::{error::StoreError, store::Store, util};
use noxious::{proxy::{ProxyConfig, ProxyRunner}, socket::TcpListener, toxic::Toxic};
use responses::*;
use serde_json::Value as JsonValue;
use std::convert::Infallible;
use std::future::Future;
use tracing::{instrument, warn};
use warp::http::StatusCode;
use warp::Reply;

/// Remove all toxics from all proxies
#[instrument(level = "info")]
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
#[instrument(level = "info", skip(store))]
pub async fn populate(configs: Vec<ProxyConfig>, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result_with_status(
        async move { store.populate::<TcpListener, ProxyRunner>(configs).await },
        StatusCode::CREATED,
    )
    .await
}

/// Get a key-value map of all proxies and their toxics in the system
#[instrument(level = "info", skip(store))]
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
    use_store_result(result, StatusCode::OK)
}

/// Create a proxy, return it if successful
#[instrument(level = "info", skip(store))]
pub async fn create_proxy(proxy: ProxyConfig, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result_with_status(
        async move { store.create_proxy::<TcpListener, ProxyRunner>(proxy).await },
        StatusCode::CREATED,
    )
    .await
}

#[instrument(level = "info", skip(store))]
pub async fn get_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_proxy(&name).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn update_proxy(
    name: String,
    new_proxy: ProxyConfig,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.update_proxy::<TcpListener, ProxyRunner>(name, new_proxy).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn remove_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result_no_content(async move { store.remove_proxy(&name).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn get_toxics(proxy_name: String, store: Store) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_toxics(&proxy_name).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn create_toxic(
    proxy_name: String,
    toxic: Toxic,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.create_toxic(proxy_name, toxic).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn get_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.get_toxic(&proxy_name, &toxic_name).await }).await
}

#[instrument(level = "info", skip(store))]
pub async fn update_toxic(
    proxy_name: String,
    toxic_name: String,
    new_toxic: Toxic,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result(async move { store.update_toxic(proxy_name, toxic_name, new_toxic).await })
        .await
}

#[instrument(level = "info", skip(store))]
pub async fn remove_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    wrap_store_result_no_content(async move { store.remove_toxic(proxy_name, toxic_name).await })
        .await
}

#[instrument(level = "info")]
pub async fn get_version() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        util::get_version(),
        StatusCode::OK,
    ))
}

mod responses {
    use serde::Serialize;
    use warp::reply::{json as json_reply, with_status};

    use crate::error::{ApiErrorResponse, StoreError};

    use super::*;

    pub async fn wrap_store_result(
        f: impl Future<Output = Result<impl Serialize, StoreError>>,
    ) -> Result<impl Reply, Infallible> {
        use_store_result(f.await, StatusCode::OK)
    }

    pub async fn wrap_store_result_with_status(
        f: impl Future<Output = Result<impl Serialize, StoreError>>,
        status_code: StatusCode,
    ) -> Result<impl Reply, Infallible> {
        use_store_result(f.await, status_code)
    }

    pub async fn wrap_store_result_no_content(
        f: impl Future<Output = Result<(), StoreError>>,
    ) -> Result<impl Reply, Infallible> {
        use_store_result_no_content(f.await)
    }

    pub fn use_store_result(
        result: Result<impl Serialize, StoreError>,
        status_code: StatusCode,
    ) -> Result<impl Reply, Infallible> {
        match result {
            Ok(data) => Ok(with_status(json_reply(&data), status_code).into_response()),
            Err(err) => {
                let data: ApiErrorResponse = err.into();
                Ok(data.into())
            }
        }
    }

    pub fn use_store_result_no_content(
        result: Result<(), StoreError>,
    ) -> Result<impl Reply, Infallible> {
        match result {
            Ok(_) => Ok(StatusCode::NO_CONTENT.into_response()),
            Err(err) => {
                let data: ApiErrorResponse = err.into();
                Ok(data.into())
            }
        }
    }
}
