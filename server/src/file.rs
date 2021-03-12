use noxious::{
    proxy::{ProxyConfig, Runner},
    socket::SocketListener,
};
use std::io;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::{error, info};

use crate::store::Store;

pub async fn get_proxy_configs(file_path: &str) -> io::Result<Vec<ProxyConfig>> {
    let mut file = File::open(file_path).await?;
    // The Toxiproxy config file is one big array and it looks like serde_json's
    // streaming API doesn't support that. So we just read the whole file into the
    // memory and parse it at once. This should be fine since the config file is not
    // expected to be large.
    // Related issue: https://github.com/serde-rs/json/issues/404

    let mut buffer = Vec::new();

    // read the whole file
    file.read_to_end(&mut buffer).await?;

    let proxy_configs: Vec<ProxyConfig> = serde_json::from_slice(&buffer)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(proxy_configs)
}

pub fn populate_initial_proxy_configs<L, R>(file_path: &str, store: Store)
where
    L: SocketListener + 'static,
    R: Runner + 'static,
{
    let file_path = file_path.to_owned();
    tokio::spawn(async move {
        match get_proxy_configs(&file_path).await {
            Ok(configs) => {
                let length = configs.len();
                if let Err(err) = store.populate::<L, R>(configs).await {
                    error!(err = ?err, "Failed to populate store from proxy configs");
                } else {
                    let config: &str = &file_path;
                    info!(config, proxies = length, "Populated proxies from file");
                }
            }
            Err(err) => {
                error!(config = ?&file_path, err = ?err, "Error reading config file");
            }
        }
    });
}
