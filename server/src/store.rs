use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

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

impl Store {
    pub fn new(sender: RequestSender<ProxyEvent, ProxyEventResult>) -> Self {
        Store {
            shared: Arc::new(Shared::new()),
            sender,
        }
    }

    #[instrument]
    pub fn reset_state(&self) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    #[instrument]
    pub fn populate(&self) -> Result<(), Box<dyn Error>> {
        // TODO: parse the json file, deserialize all toxics, start proxy tasks
        todo!()
    }

    #[instrument]
    pub fn get_proxy(name: &str) -> Result<SharedProxyInfo, NotFoundError> {
        todo!()
    }

    #[instrument]
    pub fn get_toxic(name: &str) -> Result<SerializableToxic, NotFoundError> {
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

#[derive(Debug, Clone)]
pub struct SerializableToxic {
    name: String,
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
