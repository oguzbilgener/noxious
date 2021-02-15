use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    kind: ToxicKind,            // json: attributes
    name: String,               // json: name
    toxic_type: String,         // json: type
    toxicity: f32,              // json: toxicity
    direction: StreamDirection, // excluded from json
    index: usize,               // excluded from Json
    buffer_size: usize,         // excluded from json
}

impl PartialEq for Toxic {
    fn eq(&self, other: &Self) -> bool {
        return self.name == other.name
    }
}