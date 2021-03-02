use futures::{future::join_all, stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

use crate::error::{ProxyEventError, ResourceKind, StoreError};
use bmrng::{channel, error::RequestError, RequestSender};
use noxious::{
    error::NotFoundError,
    proxy::{initialize_proxy, run_proxy, ProxyConfig, Toxics},
    signal::{Stop, Stopper},
    state::SharedProxyInfo,
    toxic::{Toxic, ToxicEvent, ToxicEventKind, ToxicEventResult},
};
use tracing::{info, instrument};

const TOXIC_EVENT_BUFFER_SIZE: usize = 2;

#[derive(Debug, Clone)]
pub struct ProxyHandle {
    /// The inner proxy state containing the added toxics and connected clients and their links
    info: SharedProxyInfo,
    /// The stopper handle to send a stop signal to this proxy and all its link tasks
    proxy_stopper: Stopper,
    /// Internal: used to check if two proxy handles are equal
    id: usize,
    /// Proxy-specific event sender
    event_sender: RequestSender<ToxicEvent, ToxicEventResult>,
}

#[derive(Debug)]
pub struct Shared {
    state: Mutex<State>,
    next_proxy_id: AtomicUsize,
    stop: Stop,
}

#[derive(Debug, Clone)]
pub struct State {
    proxies: HashMap<String, ProxyHandle>,
}

#[derive(Debug, Clone)]
pub struct Store {
    shared: Arc<Shared>,
}

type Result<T> = core::result::Result<T, StoreError>;

impl Store {
    pub fn new(stop: Stop) -> Self {
        Store {
            shared: Arc::new(Shared::new(stop)),
        }
    }

    /// Remove all toxics from all proxies
    #[instrument]
    pub async fn reset_state(&self) -> Result<()> {
        let pairs: Vec<(String, RequestSender<ToxicEvent, ToxicEventResult>)> = self
            .shared
            .get_state()
            .proxies
            .iter_mut()
            .map(|(name, handle)| (name.to_owned(), handle.event_sender.clone()))
            .collect();

        stream::iter(pairs)
            .for_each(|(name, sender)| async move {
                let _ = sender
                    .send(ToxicEvent::new(name, ToxicEventKind::RemoveAllToxics))
                    .await;
            })
            .await;

        Ok(())
    }

    #[instrument(level="trace")]
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
            match self.shared.create_proxy(config).await {
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

    #[instrument(level="trace")]
    pub async fn get_proxies(&self) -> Result<Vec<ProxyWithToxics>> {
        let state = self.shared.get_state();
        Ok(state
            .proxies
            .iter()
            .map(|(_, handle)| ProxyWithToxics::from_shared_proxy_info(handle.info.clone()))
            .collect())
    }

    #[instrument(level="trace")]
    pub async fn create_proxy(&self, config: ProxyConfig) -> Result<ProxyWithToxics> {
        let shared_proxy_info = self.shared.create_proxy(config).await?;
        Ok(ProxyWithToxics::from_shared_proxy_info(shared_proxy_info))
    }

    #[instrument(level="trace")]
    pub async fn get_proxy(&self, proxy_name: &str) -> Result<ProxyWithToxics> {
        let state = self.shared.get_state();
        let proxy = state
            .proxies
            .get(proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?;
        Ok(ProxyWithToxics::from_shared_proxy_info(proxy.info.clone()))
    }

    #[instrument(level="trace")]
    pub async fn update_proxy(
        &self,
        proxy_name: String,
        new_config: ProxyConfig,
    ) -> Result<ProxyWithToxics> {
        self.shared.remove_proxy(&proxy_name).await?;
        let shared_proxy_info = self.shared.create_proxy(new_config).await?;
        Ok(ProxyWithToxics::from_shared_proxy_info(shared_proxy_info))
    }

    #[instrument(level="trace")]
    pub async fn remove_proxy(&self, proxy_name: &str) -> Result<()> {
        self.shared.remove_proxy(proxy_name).await?;
        Ok(())
    }

    #[instrument(level="trace")]
    pub async fn create_toxic(&self, proxy_name: String, toxic: Toxic) -> Result<Toxic> {
        let sender = self
            .shared
            .get_state()
            .proxies
            .get(&proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?
            .event_sender
            .clone();

        let result = sender
            .send_receive(ToxicEvent::new(
                proxy_name,
                ToxicEventKind::AddToxic(toxic.clone()),
            ))
            .await
            .map_err(|_| StoreError::ProxyClosed);

        match result {
            Ok(_) => Ok(toxic),
            Err(err) => Err(err.into()),
        }
    }

    #[instrument(level="trace")]
    pub async fn get_toxics(&self, proxy_name: &str) -> Result<Vec<Toxic>> {
        Ok(self
            .shared
            .get_state()
            .proxies
            .get(proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?
            .info
            .state
            .lock()
            .toxics
            .to_owned()
            .into_vec())
    }

    #[instrument(level="trace")]
    pub async fn get_toxic(&self, proxy_name: &str, toxic_name: &str) -> Result<Toxic> {
        self.shared
            .get_state()
            .proxies
            .get(proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?
            .info
            .state
            .lock()
            .toxics
            .find_by_name(toxic_name)
            .ok_or(StoreError::NotFound(ResourceKind::Toxic))
    }

    #[instrument(level="trace")]
    pub async fn update_toxic(
        &self,
        proxy_name: String,
        toxic_name: String,
        toxic: Toxic,
    ) -> Result<Toxic> {
        let sender = self
            .shared
            .get_state()
            .proxies
            .get(&proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?
            .event_sender
            .clone();

        let result = sender
            .send_receive(ToxicEvent::new(
                proxy_name,
                ToxicEventKind::UpdateToxic(toxic.clone()),
            ))
            .await
            .map_err(|_| StoreError::ProxyClosed);

        match result {
            Ok(_) => Ok(toxic),
            Err(err) => Err(err.into()),
        }
    }

    #[instrument(level="trace")]
    pub async fn remove_toxic(&self, proxy_name: String, toxic_name: String) -> Result<()> {
        let sender = self.shared.get_event_sender_for_proxy(&proxy_name)?;

        let result = sender
            .send_receive(ToxicEvent::new(
                proxy_name,
                ToxicEventKind::RemoveToxic(toxic_name),
            ))
            .await
            .map_err(|_| StoreError::ProxyClosed);

        match result {
            Ok(_) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

impl Shared {
    pub fn new(stop: Stop) -> Self {
        Shared {
            state: Mutex::new(State::new()),
            next_proxy_id: AtomicUsize::new(0),
            stop,
        }
    }

    #[instrument(level="trace")]
    fn get_state(&self) -> MutexGuard<State> {
        self.state
            .lock()
            .expect("Failed to lock shared state, poison error")
    }

    async fn create_proxy(self: &Arc<Self>, config: ProxyConfig) -> Result<SharedProxyInfo> {
        if self.get_state().proxy_exists(&config.name) {
            return Err(StoreError::AlreadyExists);
        }
        let proxy_name = config.name.clone();
        let (listener, proxy_info) = initialize_proxy(config, Toxics::noop()).await?;
        let info = proxy_info.clone();
        let shared = self.clone();
        let (stop, proxy_stopper) = self.stop.fork();
        let (event_sender, event_receiver) = bmrng::channel(TOXIC_EVENT_BUFFER_SIZE);
        let launch_id = shared.next_proxy_id.fetch_add(1, Ordering::Relaxed);

        shared.get_state().proxies.insert(
            proxy_name.clone(),
            ProxyHandle {
                info: info.clone(),
                proxy_stopper,
                id: launch_id,
                event_sender,
            },
        );

        tokio::spawn(async move {
            let _ = run_proxy(listener, info, event_receiver, stop).await;
            // Proxy task ended because of the stop signal, or an I/O error on accept.
            // So we should self-clean by removing the proxy from the state, if
            // the proxy in the state with the same name also has the same launch ID.
            let mut state = shared.get_state();
            if let Some(proxy_handle) = state.proxies.get(&proxy_name) {
                if proxy_handle.id == launch_id {
                    state.proxies.remove(&proxy_name);
                }
            }
        });

        Ok(proxy_info)
    }

    async fn remove_proxy(self: &Arc<Self>, proxy_name: &str) -> Result<SharedProxyInfo> {
        if let Some(handle) = self.get_state().proxies.remove(proxy_name) {
            handle.proxy_stopper.stop();
            Ok(handle.info)
        } else {
            Err(StoreError::NotFound(ResourceKind::Proxy))
        }
    }

    fn get_event_sender_for_proxy(
        &self,
        proxy_name: &str,
    ) -> Result<RequestSender<ToxicEvent, ToxicEventResult>> {
        Ok(self
            .get_state()
            .proxies
            .get(proxy_name)
            .ok_or(StoreError::NotFound(ResourceKind::Proxy))?
            .event_sender
            .clone())
    }
}

impl State {
    fn new() -> Self {
        State {
            proxies: HashMap::new(),
        }
    }

    fn proxy_exists(&self, name: &str) -> bool {
        self.proxies.contains_key(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyWithToxics {
    pub proxy: ProxyConfig,
    pub toxics: Vec<Toxic>,
}

impl ProxyWithToxics {
    /// Create the full ProxyWithToxics from SharedProxyInfo
    pub fn from_shared_proxy_info(info: SharedProxyInfo) -> Self {
        let proxy_state = info.state.lock();
        ProxyWithToxics {
            proxy: info.clone_config(),
            toxics: proxy_state.toxics.clone().into_vec(),
        }
    }

    /// Create a new ProxyWithToxics with empty toxics
    pub fn from_proxy_config(proxy_config: ProxyConfig) -> Self {
        ProxyWithToxics {
            proxy: proxy_config,
            toxics: Vec::new(),
        }
    }
}
