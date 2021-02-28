use futures::{stream, StreamExt};
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
    proxy::{ProxyConfig, Toxics},
    signal::Stopper,
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
    info: SharedProxyInfo,
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
        // Stop the existing proxies with the same name
        {
            let mut state = self.shared.get_state();
            input
                .iter()
                .filter_map(|config| state.proxies.remove(&config.name))
                .for_each(|handle| handle.proxy_stopper.stop());
        }

        tokio::task::yield_now().await;

        let shared = self.shared.clone();

        // Since Tokio's `TcpListener::bind` sets the `SO_REUSEADDR` on the socket,
        // we probably don't need to wait until the previous proxy's TcpListener is dropped.

        stream::iter(input).for_each_concurrent(8, move |config| {
            let shared = shared.clone();
            shared.create_proxy(config)
        }).await;

        todo!()
    }

    #[instrument]
    pub async fn get_proxies(&self) -> Result<Vec<ProxyWithToxics>> {
        todo!()
    }

    #[instrument]
    pub async fn create_proxy(&self, proxy: ProxyConfig) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn get_proxy(&self, name: String) -> Result<ProxyWithToxics> {
        todo!()
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

    async fn create_proxy(self: Arc<Self>, config: ProxyConfig) {
        // TODO
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
