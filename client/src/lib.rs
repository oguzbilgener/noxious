#![forbid(unsafe_code)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]
#![cfg_attr(feature = "clippy", warn(cast_possible_truncation))]
#![cfg_attr(feature = "clippy", warn(cast_possible_wrap))]
#![cfg_attr(feature = "clippy", warn(cast_precision_loss))]
#![cfg_attr(feature = "clippy", warn(cast_sign_loss))]
#![cfg_attr(feature = "clippy", warn(missing_docs_in_private_items))]
#![cfg_attr(feature = "clippy", warn(mut_mut))]
#![cfg_attr(feature = "clippy", warn(print_stdout))]
#![cfg_attr(all(not(test), feature = "clippy"), warn(result_unwrap_used))]
#![cfg_attr(feature = "clippy", warn(unseparated_literal_suffix))]
#![cfg_attr(feature = "clippy", warn(wrong_pub_self_convention))]

//! # noxious-client

mod client;
/// The errors returned by the server
pub mod error;
mod proxy;
pub use client::Client;
pub use noxious::proxy::ProxyConfig;
pub use noxious::toxic::{StreamDirection, Toxic, ToxicKind};
pub use proxy::Proxy;
