use std::convert::Infallible;
use warp::http::StatusCode;
// use serde_derive::{Deserialize, Serialize};
use warp::{Filter, Rejection, Reply};

// #[derive(Serialize)]
// struct ErrorMessage {
//     code: u16,
//     message: String,
// }

pub async fn reset_state() -> Result<impl Reply, Infallible> {
    todo!();
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_proxy(name: String) -> Result<impl Reply, Infallible> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_toxic(proxy_name: String, toxic_name: String) -> Result<impl Reply, Infallible> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_version() -> Result<impl Reply, Infallible> {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    Ok(warp::reply::with_status(VERSION, StatusCode::OK))
}

// pub async fn respond_with_error() -> Result<impl Reply, Infallible> {
//     todo!()
// }
