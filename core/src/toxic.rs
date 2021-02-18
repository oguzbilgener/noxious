use crate::error::NotFoundError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::mem;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StreamDirection {
    #[serde(rename = "downstream")]
    Downstream,
    #[serde(rename = "upstream")]
    Upstream,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToxicKind {
    Noop,
    Latency {
        latency: u64,
        jitter: u64,
    },
    Timeout {
        timeout: i64,
    },
    Bandwidth {
        rate: i64,
    },
    SlowClose {
        delay: i64,
    },
    Slicer {
        average_size: i64,
        size_variation: i64,
        delay: i64,
    },
    LimitData {
        bytes: i64,
    },
}

#[derive(Debug, Clone)]
pub struct Toxic {
    pub(crate) kind: ToxicKind,            // json: type | attributes
    pub(crate) name: String,               // json: name
    pub(crate) toxicity: f32,              // json: toxicity
    pub(crate) direction: StreamDirection, // excluded from json
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToxicEventKind {
    ToxicAdd(Toxic),
    ToxicUpdate(Toxic),
    ToxicRemove(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ToxicEvent {
    pub(super) proxy_name: String,
    pub(super) direction: StreamDirection,
    pub(super) toxic_name: String,
    pub(super) kind: ToxicEventKind,
}

impl fmt::Display for StreamDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamDirection::Downstream => write!(f, "downstream"),
            StreamDirection::Upstream => write!(f, "upstream"),
        }
    }
}

impl PartialEq for Toxic {
    fn eq(&self, other: &Self) -> bool {
        return self.name == other.name;
    }
}

impl Toxic {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl ToxicEvent {
    pub fn new(
        proxy_name: &str,
        direction: StreamDirection,
        toxic_name: &str,
        kind: ToxicEventKind,
    ) -> Self {
        ToxicEvent {
            proxy_name: proxy_name.to_owned(),
            direction,
            toxic_name: toxic_name.to_owned(),
            kind,
        }
    }
}

pub(super) fn update_toxic_list_in_place(
    toxics: &mut Vec<Toxic>,
    event_kind: ToxicEventKind,
) -> Result<(), NotFoundError> {
    match event_kind {
        ToxicEventKind::ToxicAdd(toxic) => {
            toxics.push(toxic);
        }
        ToxicEventKind::ToxicUpdate(toxic) => {
            let old_toxic = toxics
                .iter_mut()
                .find(|el| el.get_name() == toxic.get_name())
                .ok_or(NotFoundError)?;
            let _ = mem::replace(old_toxic, toxic);
        }
        ToxicEventKind::ToxicRemove(toxic_name) => {
            let index = toxics
                .iter()
                .position(|el| el.get_name() == toxic_name)
                .ok_or(NotFoundError)?;
            toxics.remove(index);
        }
    }
    Ok(())
}
