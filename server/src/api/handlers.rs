use std::convert::Infallible;
use warp::http::StatusCode;
// use serde_derive::{Deserialize, Serialize};
use crate::store::Store;
use crate::util;
use bmrng::RequestSender;
use noxious::error::NotFoundError;
use noxious::{ToxicEvent, ToxicEventKind};
use warp::{Filter, Rejection, Reply};

// #[derive(Serialize)]
// struct ErrorMessage {
//     code: u16,
//     message: String,
// }

pub async fn reset_state(store: Store) -> Result<impl Reply, Infallible> {
    todo!();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn stub(store: Store) -> Result<impl Reply, Infallible> {
    todo!();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_proxy(name: String, store: Store) -> Result<impl Reply, Infallible> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_toxic(
    proxy_name: String,
    toxic_name: String,
    store: Store,
) -> Result<impl Reply, Infallible> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_version() -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        util::get_version(),
        StatusCode::OK,
    ))
}

// pub async fn respond_with_error() -> Result<impl Reply, Infallible> {
//     todo!()
// }
