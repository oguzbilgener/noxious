use futures::{future::join_all, stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::error::StoreError;
use bmrng::{channel, RequestSender};
use noxious::{
    error::NotFoundError,
    proxy::{initialize_proxy, run_proxy, ProxyConfig, Toxics},
    signal::{Stop, Stopper},
    state::SharedProxyInfo,
    toxic::{Toxic, ToxicEvent, ToxicEventKind},
};
use tracing::{info, instrument};

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyEvent {
    Populate {},
    CreateProxy {},
    UpdateProxy { proxy_name: String },
    RemoveProxy { proxy_name: String },
}

// TODO: this is temporary
#[derive(Debug, Clone)]
pub struct ProxyEventResult;

#[derive(Debug, Clone)]
pub struct ProxyHandle {
    /// The inner proxy state containing the added toxics and connected clients and their links
    info: SharedProxyInfo,
    /// The stopper handle to send a stop signal to this proxy and all its link tasks
    proxy_stopper: Stopper,
}

#[derive(Debug)]
pub struct Shared {
    state: Mutex<State>,
}

#[derive(Debug, Clone)]
pub struct State {
    proxies: HashMap<String, ProxyHandle>,
}

#[derive(Debug, Clone)]
pub struct Store {
    shared: Arc<Shared>,
    sender: RequestSender<ProxyEvent, ProxyEventResult>,
}

type Result<T> = core::result::Result<T, StoreError>;

impl Store {
    pub fn new(sender: RequestSender<ProxyEvent, ProxyEventResult>) -> Self {
        Store {
            shared: Arc::new(Shared::new()),
            sender,
        }
    }

    /// Remove all toxics from all proxies
    #[instrument]
    pub async fn reset_state(&self) -> Result<()> {
        let proxy_names = self.shared.get_state().get_proxy_names();
        for key in proxy_names {
            let res = self
                .sender
                .send(ProxyEvent::UpdateProxy { proxy_name: key })
                .await;
            if let Err(_) = res {
                return Err(StoreError::Other);
            }
        }
        Ok(())
    }

    #[instrument]
    pub async fn populate(&self, input: Vec<ProxyConfig>) -> Result<Vec<ProxyWithToxics>> {
        for config in &input {
            if let Err(err) = config.validate() {
                return Err(err.into());
            }
        }

        // Stop the existing proxies with the same name
        {
            let mut state = self.shared.get_state();
            input
                .iter()
                .filter_map(|config| state.proxies.remove(&config.name))
                .for_each(|handle| handle.proxy_stopper.stop());
        }

        tokio::task::yield_now().await;

        // Since Tokio's `TcpListener::bind` sets the `SO_REUSEADDR` option on the socket,
        // we probably don't need to wait until the previous proxy's TcpListener is dropped.

        let mut created_proxies = Vec::with_capacity(input.len());

        for config in input {
            match self.shared.clone().create_proxy(config).await {
                Ok(shared_proxy_info) => {
                    created_proxies.push(shared_proxy_info);
                }
                Err(err) => {
                    // Do what ToxiProxy does in `PopulateJson`, return early,
                    // even if we're halfway through creating and starting new toxics.
                    return Err(err);
                }
            }
        }

        // Newly created proxies do not yet have any toxics, so we don't need to
        // acquire any more locks to retrieve the toxic lists.
        let proxies_with_toxics: Vec<ProxyWithToxics> = created_proxies
            .into_iter()
            .map(|info| ProxyWithToxics::from_proxy_config(info.clone_config()))
            .collect();
        Ok(proxies_with_toxics)
    }

    #[instrument]
    pub async fn get_proxies(&self) -> Result<Vec<ProxyWithToxics>> {
        let state = self.shared.get_state();

        todo!()
    }

    #[instrument]
    pub async fn create_proxy(&self, proxy: ProxyConfig) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn get_proxy(&self, name: String) -> Result<ProxyWithToxics> {
        let state = self.shared.get_state();
        let proxy = state.proxies.get(&name).ok_or(StoreError::NotFound)?;
        Ok(ProxyWithToxics::from_shared_proxy_info(proxy.info.clone()))
    }

    #[instrument]
    pub async fn update_proxy(
        &self,
        name: String,
        new_config: ProxyConfig,
    ) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn remove_proxy(&self, name: String) -> Result<()> {
        todo!()
    }

    #[instrument]
    pub async fn create_toxic(
        &self,
        proxy_name: String,
        toxic: SerializableToxic,
    ) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn get_toxics(&self, proxy_name: String) -> Result<Vec<SerializableToxic>> {
        todo!()
    }

    #[instrument]
    pub async fn get_toxic(
        &self,
        proxy_name: String,
        toxic_name: String,
    ) -> Result<SerializableToxic> {
        todo!()
    }

    #[instrument]
    pub async fn update_toxic(
        &self,
        proxy_name: String,
        toxic_name: String,
        toxic: SerializableToxic,
    ) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn remove_toxic(&self, proxy_name: String, toxic_name: String) -> Result<()> {
        todo!()
    }
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            state: Mutex::new(State::new()),
        }
    }

    fn get_state(&self) -> MutexGuard<State> {
        self.state
            .lock()
            .expect("Failed to lock shared state, poison error")
    }

    async fn create_proxy(self: Arc<Self>, config: ProxyConfig) -> Result<SharedProxyInfo> {
        // TODO
        let (listener, proxy_info) = initialize_proxy(config, Toxics::noop()).await?;

        let info = proxy_info.clone();
        tokio::spawn(async move {
            // TODO: figure out this channel, make it 2-way probably
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let (global_stop, _) = Stop::new();
            run_proxy(listener, info, rx, global_stop).await
        });

        Ok(proxy_info)
    }
}

impl State {
    fn new() -> Self {
        State {
            proxies: HashMap::new(),
        }
    }

    fn get_proxy_names(&self) -> Vec<String> {
        Vec::from_iter(self.proxies.keys().map(|s| s.to_owned()))
    }

    fn get_proxy_names_set(&self) -> HashSet<String> {
        self.proxies
            .keys()
            .fold(HashSet::with_capacity(self.proxies.len()), |mut acc, s| {
                acc.insert(s.to_owned());
                acc
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableToxic {
    pub name: String,
}

impl From<SerializableToxic> for Toxic {
    fn from(_: SerializableToxic) -> Self {
        todo!()
    }
}

impl From<Toxic> for SerializableToxic {
    fn from(_: Toxic) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyWithToxics {
    pub proxy: ProxyConfig,
    pub toxics: Vec<SerializableToxic>,
}

impl ProxyWithToxics {
    /// Create a new ProxyWithToxics with empty toxics
    pub fn from_proxy_config(proxy_config: ProxyConfig) -> Self {
        ProxyWithToxics {
            proxy: proxy_config,
            toxics: Vec::new(),
        }
    }

    pub fn from_shared_proxy_info(info: SharedProxyInfo) -> Self {
        let proxy_state = info.state.lock();
        ProxyWithToxics {
            proxy: info.clone_config(),
            toxics: proxy_state.toxics.clone().into_vec(),
        }
    }
}
