use crate::error::NotFoundError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::mem;

///
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum StreamDirection {
    /// Represents an I/O channel from server to the client
    #[serde(rename = "downstream")]
    Downstream,
    /// Represents an I/O channel from the client to the server
    #[serde(rename = "upstream")]
    Upstream,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "attributes")]
/// Toxic kind and toxic-specific attributes
pub enum ToxicKind {
    #[serde(rename = "noop")]
    Noop,
    #[serde(rename = "latency")]
    Latency { latency: u64, jitter: u64 },
    #[serde(rename = "timeout")]
    Timeout { timeout: u64 },
    #[serde(rename = "bandwidth")]
    Bandwidth { rate: u64 },
    #[serde(rename = "slow_close")]
    SlowClose { delay: u64 },
    #[serde(rename = "slicer")]
    Slicer {
        average_size: u64,
        size_variation: u64,
        delay: u64,
    },
    #[serde(rename = "limit_data")]
    LimitData { bytes: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toxic {
    #[serde(flatten)]
    pub(crate) kind: ToxicKind, // json: type | attributes
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
    /// TODO
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl ToxicEvent {
    /// TODO
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

#[cfg(test)]
mod serde_tests {
    use super::*;
    use serde_json::{from_str, to_string, Error as SerdeError};

    #[test]
    fn test_noop() {
        let toxic = Toxic {
            kind: ToxicKind::Noop,
            name: "foo".to_owned(),
            toxicity: 0.67,
            direction: StreamDirection::Downstream,
        };
        let serialized = to_string(&toxic).unwrap();
        let expected =
            "{\"type\":\"noop\",\"name\":\"foo\",\"toxicity\":0.67,\"direction\":\"downstream\"}";
        assert_eq!(expected, serialized);

        let deserialized = from_str(&serialized).unwrap();
        assert_eq!(toxic, deserialized);
    }

    #[test]
    fn test_latency() {
        let toxic = Toxic {
            kind: ToxicKind::Latency {
                latency: 4321,
                jitter: 5,
            },
            name: "lat".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let serialized = to_string(&toxic).unwrap();
        let expected =
            "{\"type\":\"latency\",\"attributes\":{\"latency\":4321,\"jitter\":5},\"name\":\"lat\",\"toxicity\":1.0,\"direction\":\"upstream\"}";
        assert_eq!(expected, serialized);

        let deserialized = from_str(&serialized).unwrap();
        assert_eq!(toxic, deserialized);
    }

    #[test]
    fn test_toxicity_de_int() {
        let input =
            "{\"type\":\"noop\",\"name\":\"foo\",\"toxicity\":1,\"direction\":\"downstream\"}";
        let deserialized = from_str(&input).unwrap();
        let expected = Toxic {
            kind: ToxicKind::Noop,
            name: "foo".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_latency_de_negative() {
        let input_ok =
            "{\"type\":\"latency\",\"attributes\":{\"latency\":21,\"jitter\":0},\"name\":\"lat\",\"toxicity\":1,\"direction\":\"downstream\"}";
        let input_err =
            "{\"type\":\"latency\",\"attributes\":{\"latency\":-21,\"jitter\":0},\"name\":\"lat\",\"toxicity\":1,\"direction\":\"downstream\"}";
        let deserialized_ok: Result<Toxic, SerdeError> = from_str(&input_ok);
        let deserialized_err: Result<Toxic, SerdeError> = from_str(&input_err);

        assert_eq!(true, deserialized_ok.is_ok());
        assert_eq!(
            "invalid value: integer `-21`, expected u64 at line 1 column 109",
            deserialized_err.unwrap_err().to_string()
        );
    }
}
