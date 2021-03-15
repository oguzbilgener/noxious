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
    if err.is_not_found() {
        Ok(StatusCode::NOT_FOUND.into_response())
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        Ok(StatusCode::BAD_REQUEST.into_response())
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
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

    use crate::api::make_filters;
    use crate::store::tests::__mock_MockNoopRunner_Runner::__initialize_proxy::Context as IpContext;
    use crate::store::tests::__mock_MockNoopRunner_Runner::__run_proxy::Context as RpContext;
    use crate::store::tests::{hack_handle_id, MockNoopListener, MockNoopRunner, MOCK_LOCK};
    use crate::store::ProxyWithToxics;
    use noxious::{
        proxy::ProxyConfig,
        signal::Stop,
        state::{ProxyState, SharedProxyInfo},
        toxic::{StreamDirection, Toxic, ToxicKind},
    };
    use tokio_test::assert_ok;
    use warp::http::header::{CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT};

    use super::*;

    struct MockHandle(IpContext, RpContext);

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

    fn mock_proxy_runner(store: Store) -> MockHandle {
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

        run_ctx.expect().returning(
            move |_listener: MockNoopListener, info, event_receiver, stop, _closer| {
                hack_handle_id(store.clone(), &info);
                tokio::spawn(noxious::proxy::listen_toxic_events(
                    info.state,
                    event_receiver,
                    stop,
                    info.config,
                ));
                Ok(())
            },
        );
        MockHandle(init_ctx, run_ctx)
    }

    async fn insert_proxies(store: &Store) {
        let proxies = vec![
            ProxyConfig {
                name: "server1".to_owned(),
                listen: "127.0.0.1:5431".to_owned(),
                upstream: "127.0.0.1:5432".to_owned(),
                enabled: true,
                rand_seed: None,
            },
            ProxyConfig {
                name: "server2".to_owned(),
                listen: "127.0.0.1:27017".to_owned(),
                upstream: "127.0.0.1:27018".to_owned(),
                enabled: false,
                rand_seed: None,
            },
        ];
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
                name: "server1".to_owned(),
                listen: "127.0.0.1:5431".to_owned(),
                upstream: "127.0.0.1:5432".to_owned(),
                enabled: true,
                rand_seed: None,
            },
            ProxyConfig {
                name: "server2".to_owned(),
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
        let _handle = mock_proxy_runner(store.clone());
        insert_proxies(&store).await;

        let req = warp::test::request().method("GET").path("/proxies");
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body = reply.body();
        let proxies: HashMap<String, ProxyWithToxics> =
            serde_json::from_slice(body).expect("failed to parse body");
        assert_eq!(2, proxies.len());
        let server1 = proxies.get("server1").unwrap();
        assert_eq!("127.0.0.1:5431", server1.proxy.listen);
        assert_eq!("127.0.0.1:5432", server1.proxy.upstream);
        assert_eq!(true, server1.proxy.enabled);
        let server2 = proxies.get("server2").unwrap();
        assert_eq!("127.0.0.1:27017", server2.proxy.listen);
        assert_eq!("127.0.0.1:27018", server2.proxy.upstream);
        assert_eq!(false, server2.proxy.enabled);
    }

    #[tokio::test]
    async fn create_proxy_conflict() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = create_proxy(store.clone());
        let _handle = mock_proxy_runner(store.clone());
        insert_proxies(&store).await;

        let payload = serde_json::to_vec(&ProxyConfig {
            name: "server1".to_owned(),
            listen: "127.0.0.1:1234".to_owned(),
            upstream: "127.0.0.1:1235".to_owned(),
            enabled: true,
            rand_seed: None,
        })
        .unwrap();

        let req = warp::test::request()
            .method("POST")
            .path("/proxies")
            .header(CONTENT_TYPE, "application/json")
            .body(payload);
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::CONFLICT, reply.status());
    }

    #[tokio::test]
    async fn create_and_get_proxy() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let _handle = mock_proxy_runner(store.clone());

        let create_filter = create_proxy(store.clone());
        let create_toxic_filter = create_toxic(store.clone());
        let get_filter = get_proxy(store.clone());

        let config = ProxyConfig {
            name: "server1".to_owned(),
            listen: "127.0.0.1:1234".to_owned(),
            upstream: "127.0.0.1:1235".to_owned(),
            enabled: true,
            rand_seed: None,
        };
        let toxic = Toxic {
            kind: ToxicKind::Noop,
            name: "stub".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let payload = serde_json::to_vec(&config).unwrap();

        let req = warp::test::request()
            .method("POST")
            .path("/proxies")
            .header(CONTENT_TYPE, "application/json")
            .body(&payload);
        let reply = req.reply(&create_filter).await;
        assert_eq!(StatusCode::CREATED, reply.status());

        let payload = serde_json::to_vec(&toxic).unwrap();
        let req = warp::test::request()
            .method("POST")
            .path("/proxies/server1/toxics")
            .header(CONTENT_TYPE, "application/json")
            .body(&payload);
        let reply = req.reply(&create_toxic_filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body: Toxic = serde_json::from_slice(reply.body()).unwrap();
        assert_eq!(&toxic, &body);

        let req = warp::test::request().method("GET").path("/proxies/server1");
        let reply = req.reply(&get_filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body: ProxyWithToxics = serde_json::from_slice(reply.body()).unwrap();
        assert_eq!(&config, &body.proxy);
        assert_eq!(&toxic, &body.toxics[0]);
    }

    #[tokio::test]
    async fn test_update_proxy() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = make_filters(store.clone());
        let _handle = mock_proxy_runner(store.clone());
        insert_proxies(&store).await;

        let disabled_config = ProxyConfig {
            name: "server1".to_owned(),
            listen: "127.0.0.1:1234".to_owned(),
            upstream: "127.0.0.1:1235".to_owned(),
            enabled: false,
            rand_seed: None,
        };
        let toxic = Toxic {
            kind: ToxicKind::Noop,
            name: "stub".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };

        // Create a toxic to make sure the response body of update includes the toxic too
        let payload = serde_json::to_vec(&toxic).unwrap();
        let req = warp::test::request()
            .method("POST")
            .path("/proxies/server1/toxics")
            .header(CONTENT_TYPE, "application/json")
            .body(&payload);
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body: Toxic = serde_json::from_slice(reply.body()).unwrap();
        assert_eq!(&toxic, &body);

        let payload = serde_json::to_vec(&disabled_config).unwrap();

        let req = warp::test::request()
            .method("POST")
            .path("/proxies/server1")
            .header(CONTENT_TYPE, "application/json")
            .body(&payload);
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::OK, reply.status());
        let body: ProxyWithToxics = serde_json::from_slice(reply.body()).unwrap();
        assert_eq!(&disabled_config, &body.proxy);
        assert_eq!(&toxic, &body.toxics[0]);
    }

    #[tokio::test]
    async fn test_remove_proxy() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = make_filters(store.clone());
        let _handle = mock_proxy_runner(store.clone());
        insert_proxies(&store).await;
        let req = warp::test::request()
            .method("DELETE")
            .path("/proxies/server1");
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::NO_CONTENT, reply.status());
    }

    #[tokio::test]
    async fn test_remove_proxy_not_found() {
        let _lock = MOCK_LOCK.lock().await;
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = make_filters(store.clone());
        let _handle = mock_proxy_runner(store.clone());
        insert_proxies(&store).await;
        let req = warp::test::request()
            .method("DELETE")
            .path("/proxies/blah");
        let reply = req.reply(&filter).await;
        assert_eq!(StatusCode::NOT_FOUND, reply.status());
    }

    #[tokio::test]
    async fn version_filter() {
        let filter = version();
        let reply = warp::test::request()
            .method("GET")
            .path("/version")
            .reply(&filter)
            .await;
        let body = std::str::from_utf8(reply.body()).unwrap();
        assert_eq!(crate::util::get_version(), body);
    }

    #[tokio::test]
    async fn allows_custom_clients() {
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = make_filters(store);
        let reply = warp::test::request()
            .method("GET")
            .path("/version")
            .header(USER_AGENT, "mybrowser/1.0")
            .reply(&filter)
            .await;
        assert_eq!(StatusCode::OK, reply.status());
    }

    #[tokio::test]
    async fn disallows_firefox() {
        let agent = "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:86.0) Gecko/20100101 Firefox/86.0";
        let (stop, _stopper) = Stop::new();
        let store = Store::new(stop, None);
        let filter = make_filters(store);
        let reply = warp::test::request()
            .method("GET")
            .path("/version")
            .header(USER_AGENT, agent)
            .reply(&filter)
            .await;
        assert_eq!(StatusCode::FORBIDDEN, reply.status());
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
