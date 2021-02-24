use crate::api::handlers;
use warp::http::StatusCode;
use std::convert::Infallible;
use warp::http::header::{CONTENT_TYPE, ORIGIN, USER_AGENT};
use warp::{Filter, Rejection, Reply};

// pub fn proxies() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
//     todo!()
// }

/// POST /reset
pub fn reset() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("reset")
        .and(warp::post())
        .and(util::empty_body())
        .and_then(handlers::reset_state)
}

/// POST /populate
pub fn populate() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("populate")
        .and(warp::post())
        .and_then(handlers::reset_state)
}

/// GET /proxies
pub fn get_proxies() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::get())
        .and_then(handlers::reset_state)
}

/// POST /proxies
pub fn create_proxy() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::post())
        .and_then(handlers::reset_state)
}

/// GET /proxies/{proxy}
pub fn get_proxy() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::get())
        .and_then(handlers::get_proxy)
}

/// POST /proxies/{proxy}
pub fn update_proxy() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Note: This is not very RESTful but we're just following the original Toxiproxy API spec.
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::post())
        .and_then(handlers::get_proxy)
}

/// DELETE /proxies/{proxy}
pub fn delete_proxy() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::get())
        .and_then(handlers::get_proxy)
}

/// GET /proxies/{proxy}/toxics
pub fn get_toxics() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::get())
        .and_then(handlers::get_proxy)
}

/// POST /proxies/{proxy}/toxics
pub fn create_toxics() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::post())
        .and_then(handlers::get_proxy)
}

/// GET /proxies/{proxy}/toxics/{toxic}
pub fn get_toxic() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::get())
        .and_then(handlers::get_toxic)
}

/// POST /proxies/{proxy}/toxics/{toxic}
pub fn update_toxic() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::post())
        .and_then(handlers::get_toxic)
}

/// DELETE /proxies/{proxy}/toxics/{toxic}
pub fn delete_toxic() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("proxies")
        .and(warp::path::param())
        .and(warp::path("toxics"))
        .and(warp::path::param())
        .and(warp::post())
        .and_then(handlers::get_toxic)
}
/// GET /version

pub fn version() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path("version")
        // .and(warp::get().or(warp::head()))
        .and_then(handlers::get_version)
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

pub(crate) mod util {
    use super::*;
    pub(super) fn empty_body() -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(0)
    }

}
