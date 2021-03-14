use crate::api::handlers;
use crate::error::{ApiErrorResponse, StoreError};
use crate::store::Store;
use std::convert::Infallible;
use tracing::{debug, error};
use warp::http::header::USER_AGENT;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

/// POST /reset
pub fn reset(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("reset"))
        .and(warp::path::end())
        .and(util::empty_body())
        .and(util::add_store(store))
        .and_then(handlers::reset_state)
}

/// POST /populate
pub fn populate(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("populate"))
        .and(warp::path::end())
        .and(util::parse_body())
        .and(util::add_store(store))
        .and_then(handlers::populate)
}

/// GET /proxies
pub fn get_proxies(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("proxies"))
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::get_proxies)
}

/// POST /proxies
pub fn create_proxy(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("proxies"))
        .and(warp::path::end())
        .and(util::parse_body())
        .and(util::add_store(store))
        .and_then(handlers::create_proxy)
}

/// GET /proxies/{proxy}
pub fn get_proxy(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::get_proxy)
}

/// POST /proxies/{proxy}
pub fn update_proxy(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Note: This is not very RESTful but we're just following the original Toxiproxy API spec.
    warp::post()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::parse_body())
        .and(util::add_store(store))
        .and_then(handlers::update_proxy)
}

/// DELETE /proxies/{proxy}
pub fn remove_proxy(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::remove_proxy)
}

/// GET /proxies/{proxy}/toxics
pub fn get_toxics(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::get_toxics)
}

/// POST /proxies/{proxy}/toxics
pub fn create_toxic(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::end())
        .and(util::parse_body())
        .and(util::add_store(store))
        .and_then(handlers::create_toxic)
}

/// GET /proxies/{proxy}/toxics/{toxic}
pub fn get_toxic(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::get_toxic)
}

/// POST /proxies/{proxy}/toxics/{toxic}
pub fn update_toxic(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::parse_body())
        .and(util::add_store(store))
        .and_then(handlers::update_toxic)
}

/// DELETE /proxies/{proxy}/toxics/{toxic}
pub fn remove_toxic(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("proxies"))
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::path::end())
        .and(util::add_store(store))
        .and_then(handlers::remove_toxic)
}
/// GET /version

pub fn version() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("version").and_then(handlers::get_version)
}

pub fn disallow_browsers() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::any()
        .and(warp::header::header(USER_AGENT.as_str()))
        .and_then(move |user_agent: String| async move {
            if user_agent.starts_with("Mozilla/") {
                Ok(warp::reply::with_status(
                    "User agent not allowed",
                    StatusCode::FORBIDDEN,
                ))
            } else {
                Err(warp::reject::reject())
            }
        })
}

pub async fn handle_errors(err: Rejection) -> Result<impl Reply, Infallible> {
    dbg!(&err);
    if err.is_not_found() {
        Ok(StatusCode::NOT_FOUND.into_response())
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        Ok(StatusCode::BAD_REQUEST.into_response())
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        // This is a bug in Warp, it somehow rejects with MethodNotAllowed for all routes, even those that match the method
        // - https://github.com/seanmonstar/warp/issues/77
        // - https://github.com/seanmonstar/warp/issues/451
        debug!(err = ?err, "Got method not allowed");
        let err = ApiErrorResponse::new("Not found", StatusCode::NOT_FOUND);
        Ok(err.into())
    } else {
        error!(err = ?err, "Unhandled error");
        let err = StoreError::Other;
        let err: ApiErrorResponse = err.into();
        Ok(err.into())
    }
}

pub(crate) mod util {
    use serde::de::DeserializeOwned;

    use super::*;
    pub(super) fn empty_body() -> impl Filter<Extract = (), Error = Rejection> + Clone {
        warp::body::content_length_limit(0)
    }

    pub(super) fn parse_body<T: DeserializeOwned + Send>(
    ) -> impl Filter<Extract = (T,), Error = Rejection> + Clone {
        // Body size up to 64 KB
        warp::body::content_length_limit(1024 * 64).and(warp::body::json::<T>())
    }

    pub(super) fn add_store(
        store: Store,
    ) -> impl Filter<Extract = (Store,), Error = Infallible> + Clone {
        warp::any().map(move || store.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use crate::store::tests::{hack_handle_id, MockNoopListener, MockNoopRunner, MOCK_LOCK};
    use crate::store::ProxyWithToxics;
    use noxious::{
        proxy::ProxyConfig,
        signal::Stop,
        state::{ProxyState, SharedProxyInfo},
    };
    use tokio_test::assert_ok;
    use warp::http::header::CONTENT_LENGTH;

    use super::*;

    #[tokio::test]
    async fn reset_filter() {
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = reset(store);
        let req = warp::test::request()
            .method("POST")
            .path("/reset")
            .header(CONTENT_LENGTH, 0);
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::NO_CONTENT, reply.status());
    }

    async fn insert_proxies(store: Store) {
        let proxies = vec![
            ProxyConfig {
                name: "foo".to_owned(),
                listen: "127.0.0.1:5431".to_owned(),
                upstream: "127.0.0.1:5432".to_owned(),
                enabled: true,
                rand_seed: None,
            },
            ProxyConfig {
                name: "bar".to_owned(),
                listen: "127.0.0.1:27017".to_owned(),
                upstream: "127.0.0.1:27018".to_owned(),
                enabled: false,
                rand_seed: None,
            },
        ];
        let init_ctx = MockNoopRunner::initialize_proxy_context();
        let run_ctx = MockNoopRunner::run_proxy_context();
        init_ctx.expect().returning(|config, initial_toxics| {
            let listener = MockNoopListener::default();
            let proxy_info = SharedProxyInfo {
                state: Arc::new(ProxyState::new(initial_toxics)),
                config: Arc::new(config),
            };
            Ok((listener, proxy_info))
        });

        // let (done, mark_done) = Close::new();
        let st2 = store.clone();
        run_ctx.expect().returning(
            move |_listener: MockNoopListener, info, _event_receiver, _stop, _closer| {
                hack_handle_id(st2.clone(), &info);
                Ok(())
            },
        );
        assert_ok!(
            store
                .populate::<MockNoopListener, MockNoopRunner>(proxies)
                .await
        );
    }

    #[tokio::test]
    async fn populate_filter() {
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = populate(store);
        let proxies = vec![
            ProxyConfig {
                name: "foo".to_owned(),
                listen: "127.0.0.1:5431".to_owned(),
                upstream: "127.0.0.1:5432".to_owned(),
                enabled: true,
                rand_seed: None,
            },
            ProxyConfig {
                name: "bar".to_owned(),
                listen: "127.0.0.1:27017".to_owned(),
                upstream: "127.0.0.1:27018".to_owned(),
                enabled: false,
                rand_seed: None,
            },
        ];
        let body = serde_json::to_vec(&proxies).unwrap();
        let req = warp::test::request()
            .method("POST")
            .path("/populate")
            .body(&body);
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::CREATED, reply.status());
    }

    #[tokio::test]
    async fn get_proxies_empty() {
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = get_proxies(store);
        let req = warp::test::request().method("GET").path("/proxies");
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body = reply.body();
        let proxies: HashMap<String, ProxyWithToxics> =
            serde_json::from_slice(body).expect("failed to parse body");
        assert_eq!(0, proxies.len());
    }

    #[tokio::test]
    async fn get_proxies_some() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = get_proxies(store.clone());

        insert_proxies(store).await;

        let req = warp::test::request().method("GET").path("/proxies");
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body = reply.body();
        let proxies: HashMap<String, ProxyWithToxics> =
            serde_json::from_slice(body).expect("failed to parse body");
        assert_eq!(2, proxies.len());
        let foo = proxies.get("foo").unwrap();
        assert_eq!("127.0.0.1:5431", foo.proxy.listen);
        assert_eq!("127.0.0.1:5432", foo.proxy.upstream);
        assert_eq!(true, foo.proxy.enabled);
        let bar = proxies.get("bar").unwrap();
        assert_eq!("127.0.0.1:27017", bar.proxy.listen);
        assert_eq!("127.0.0.1:27018", bar.proxy.upstream);
        assert_eq!(false, bar.proxy.enabled);
    }

    #[tokio::test]
    async fn empty_body_filter() {
        let filter = util::empty_body();
        assert_eq!(
            false,
            warp::test::request()
                .method("POST")
                .body("stuff")
                .matches(&filter)
                .await
        );
        assert_eq!(
            true,
            warp::test::request()
                .method("POST")
                .body(b"")
                .matches(&filter)
                .await
        );
    }

    #[tokio::test]
    async fn body_limit_filter() {
        let filter = util::parse_body::<Vec<u8>>();
        let large_body: [u8; 1024 * 65] = [1; 1024 * 65];
        assert_eq!(
            false,
            warp::test::request()
                .method("POST")
                .body(large_body)
                .matches(&filter)
                .await
        );
    }
}
