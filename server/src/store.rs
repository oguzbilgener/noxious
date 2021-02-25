use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bmrng::{channel, RequestSender};
use noxious::error::NotFoundError;
use noxious::{ProxyConfig, ToxicEvent, ToxicEventKind, Toxics};

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

#[derive(Debug, Clone)]
pub struct ProxyInfo {
    config: ProxyConfig,
    /// This is for informational purposes only, the actual e
    toxics: Toxics,
}

#[derive(Debug)]
pub struct Shared {
    state: Mutex<HashMap<String, ProxyInfo>>,
}

#[derive(Debug, Clone)]
pub struct Store {
    shared: Arc<Shared>,
    sender: RequestSender<ProxyEvent, ProxyEventResult>,
}
