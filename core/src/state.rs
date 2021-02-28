use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::Mutex as AsyncMutex;

use crate::{
    proxy::{Links, ProxyConfig, Toxics},
    toxic::{Toxic, ToxicKind},
};

/// TODO
#[derive(Debug)]
pub struct ProxyState {
    inner: Mutex<ProxyStateInner>,
}

/// TODO
#[derive(Debug)]
pub struct ProxyStateInner {
    /// Socket address -> (Upstream, Downstream)
    pub clients: HashMap<SocketAddr, Links>,
    /// The collection of toxics active over upstream and downstream connections
    pub toxics: Toxics,
}

/// The proxy config and state to allow the API server read from it.
/// The config is immutable, but the state is behind a mutex.
#[derive(Debug, Clone)]
pub struct SharedProxyInfo {
    /// The current proxy state containing the current collection of toxics and the connected clients
    pub state: Arc<ProxyState>,
    /// The immutable essential proxy config, like the proxy name, upstream and downstream addresses
    pub config: Arc<ProxyConfig>,
}

impl ProxyState {
    pub(crate) fn new(toxics: Toxics) -> Self {
        ProxyState {
            inner: Mutex::new(ProxyStateInner {
                clients: HashMap::new(),
                toxics,
            }),
        }
    }

    /// Get the inner state, or panic if the lock is poisoned
    pub fn lock(&self) -> MutexGuard<ProxyStateInner> {
        self.inner.lock().expect("ProxyState poisoned")
    }
}

/// The state for stateful toxics
#[derive(Debug, PartialEq)]
pub enum ToxicState {
    LimitData { bytes_transmitted: usize },
}

impl ToxicState {
    /// Initialize a ToxicState for the ToxicKind, if the ToxicKind is stateful
    pub fn for_toxic_kind(kind: &ToxicKind) -> Option<ToxicState> {
        match kind {
            ToxicKind::LimitData { .. } => Some(ToxicState::LimitData {
                bytes_transmitted: 0,
            }),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ToxicStateHolder {
    inner: Mutex<HashMap<String, Arc<AsyncMutex<ToxicState>>>>,
}

impl ToxicStateHolder {
    pub(crate) fn for_toxics(toxics: &Toxics) -> Option<Arc<ToxicStateHolder>> {
        let toxics_pair: [&[Toxic]; 2] = [&toxics.upstream, &toxics.downstream];
        let stateful_toxics: Vec<&Toxic> = toxics_pair
            .iter()
            .flat_map(|direction_toxics| {
                direction_toxics
                    .iter()
                    .filter(|toxic| toxic.kind.is_stateful())
            })
            .collect();

        if stateful_toxics.is_empty() {
            None
        } else {
            let mut state_map: HashMap<String, Arc<AsyncMutex<ToxicState>>> = HashMap::new();

            for toxic in stateful_toxics {
                if let Some(initial_toxic_state) = ToxicState::for_toxic_kind(&toxic.kind) {
                    state_map.insert(
                        toxic.name.to_owned(),
                        Arc::new(AsyncMutex::new(initial_toxic_state)),
                    );
                }
            }
            Some(Arc::new(ToxicStateHolder {
                inner: Mutex::new(state_map),
            }))
        }
    }

    pub(crate) fn get_state_for_toxic(
        &self,
        toxic_name: &str,
    ) -> Option<Arc<AsyncMutex<ToxicState>>> {
        let inner = self.inner.lock().expect("ToxicStateHolder lock poisoned");
        inner.get(toxic_name).map(|toxic| Arc::clone(toxic))
    }
}
