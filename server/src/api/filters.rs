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
        .and(util::empty_body())
        .and(util::add_store(store))
        .and_then(handlers::reset_state)
}

/// POST /populate
pub fn populate(store: Store) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::post()
        .and(warp::path("populate"))
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
