use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use crate::error::StoreError;
use bmrng::{channel, RequestSender};
use noxious::{error::NotFoundError, SharedProxyInfo, Toxic};
use noxious::{ProxyConfig, ToxicEvent, ToxicEventKind, Toxics};
use tracing::{info, instrument};

#[derive(Debug, Clone, PartialEq)]
pub enum ProxyEvent {
    ResetToxics {},
    Populate {},
    CreateProxy {},
    UpdateProxy {},
    RemoveProxy {},
}

#[derive(Debug, Clone)]
pub struct ProxyEventResult;

#[derive(Debug)]
pub struct Shared {
    state: Mutex<HashMap<String, SharedProxyInfo>>,
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

    #[instrument]
    pub async fn reset_state(&self) -> Result<()> {
        todo!()
    }

    #[instrument]
    pub async fn populate(&self, input: Vec<ProxyConfig>) -> Result<Vec<ProxyWithToxics>> {
        // TODO: parse the json file, deserialize all toxics, start proxy tasks
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
    pub async fn update_proxy(&self, name: String, new_config: ProxyConfig) -> Result<ProxyWithToxics> {
        todo!()
    }

    #[instrument]
    pub async fn remove_proxy(&self, name: String) -> Result<()> {
        todo!()
    }

    #[instrument]
    pub async fn create_toxic(&self, proxy_name: String, toxic: SerializableToxic) -> Result<ProxyWithToxics> {
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
    pub async fn update_toxic(&self, proxy_name: String, toxic_name: String, toxic: SerializableToxic) -> Result<ProxyWithToxics> {
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
            state: Mutex::new(HashMap::new()),
        }
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
