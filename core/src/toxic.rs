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
        timeout: u64,
    },
    Bandwidth {
        rate: u64,
    },
    SlowClose {
        delay: u64,
    },
    Slicer {
        average_size: u64,
        size_variation: u64,
        delay: u64,
    },
    LimitData {
        bytes: u64,
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
    ToxicRemoveAll,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToxicEvent {
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

impl ToxicKind {
    pub(crate) fn has_close_logic(&self) -> bool {
        match self {
            ToxicKind::SlowClose { .. } | ToxicKind::LimitData { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn is_stateful(&self) -> bool {
        match self {
            ToxicKind::LimitData { .. } => true,
            _ => false,
        }
    }

    pub(crate) fn chunk_buffer_capacity(&self) -> usize {
        match self {
            ToxicKind::Latency { .. } => 1024,
            _ => 1,
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
        ToxicEventKind::ToxicRemoveAll => {
            toxics.clear();
        }
    }
    Ok(())
}

impl fmt::Display for Toxic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.kind)
    }
}

impl fmt::Display for ToxicKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToxicKind::Noop => {
                write!(f, "Noop")
            }
            ToxicKind::Latency { latency, .. } => {
                write!(f, "Latency({})", latency)
            }
            ToxicKind::Timeout { timeout } => {
                write!(f, "Timeout({})", timeout)
            }
            ToxicKind::Bandwidth { rate } => {
                write!(f, "Bandwidth({})", rate)
            }
            ToxicKind::SlowClose { delay } => {
                write!(f, "SlowClose({})", delay)
            }
            ToxicKind::Slicer { average_size, .. } => {
                write!(f, "Slicer({})", average_size)
            }
            ToxicKind::LimitData { bytes } => {
                write!(f, "LimitData({})", bytes)
            }
        }
    }
}
