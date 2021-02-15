use serde::{Deserialize, Serialize};
use std::fmt;

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
        latency: i64,
        jitter: i64,
        buffer_size: usize,
    },
    Timeout {
        timeout: i64,
    },
    Bandwidth {
        rate: i64,
        buffer_size: usize,
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
