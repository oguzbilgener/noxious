use std::env;

pub fn init_tracing() {
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "noxious=info");
    }
    pretty_env_logger::init();
    // tracing_log::env_logger::init();
    // LogTracer::init().expect("Failed to set a global logger");
}

pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
