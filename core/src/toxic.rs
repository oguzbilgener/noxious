use crate::error::ToxicUpdateError;
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
    /// Passes all data through without any toxic effects
    #[serde(rename = "noop")]
    Noop,
    /// Passes data through with the a delay of latency +/- jitter added
    #[serde(rename = "latency")]
    Latency {
        /// Latency to be added, in milliseconds
        latency: u64,
        /// Jitter to be added to the latency, also in milliseconds
        #[serde(default = "default_zero")]
        jitter: u64,
    },
    /// Stops any data from flowing through, and will close the connection after a timeout
    #[serde(rename = "timeout")]
    Timeout {
        /// in milliseconds
        timeout: u64,
    },
    /// Passes data through at a limited rate
    #[serde(rename = "bandwidth")]
    Bandwidth {
        /// in KB/S
        rate: u64,
    },
    /// Stops the TCP connection from closing until after a delay
    #[serde(rename = "slow_close")]
    SlowClose {
        /// in milliseconds
        delay: u64,
    },
    /// Slices data into multiple smaller packets
    #[serde(rename = "slicer")]
    Slicer {
        /// Average number of bytes to slice at
        average_size: u64,
        /// +/- bytes to vary sliced amounts. Must be less than the average size
        #[serde(default = "default_zero")]
        size_variation: u64,
        /// Microseconds to delay each packet.
        #[serde(default = "default_zero")]
        delay: u64,
    },
    /// Adds a limit of bytes transferred to the proxy session
    #[serde(rename = "limit_data")]
    LimitData {
        /// the limit
        bytes: u64,
    },
}

/// Something that can be attached to a link to modify the way the data is passed through
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Toxic {
    #[serde(flatten)]
    /// The kind which also contains kind-specific attributes
    pub kind: ToxicKind,
    /// The unique name for this toxic
    #[serde(default = "default_name")]
    pub name: String,
    /// The probability of this toxic being active
    #[serde(default = "default_toxicity")]
    pub toxicity: f32,
    #[serde(alias = "stream", default = "default_direction")]
    /// The direction this toxic is install on
    pub direction: StreamDirection,
}

/// The inners of a proxy state update event passed to the proxy runner task
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub enum ToxicEventKind {
    /// Add a new toxic to a proxy
    AddToxic(Toxic),
    /// Replace a toxic with the same name with this new toxic
    UpdateToxic(Toxic),
    /// Remove a toxic by name
    RemoveToxic(String),
    /// Reset. Remove all toxics
    RemoveAllToxics,
}

/// A proxy state update event passed to the proxy runner task
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
pub struct ToxicEvent {
    pub proxy_name: String,
    pub kind: ToxicEventKind,
}

/// The result return after the toxic event is processed. May return Ok or ToxicUpdateError
pub type ToxicEventResult = Result<(), ToxicUpdateError>;

fn default_name() -> String {
    "".to_owned()
}
fn default_toxicity() -> f32 {
    1.0
}

fn default_direction() -> StreamDirection {
    StreamDirection::Downstream
}

fn default_zero() -> u64 {
    0
}

impl fmt::Display for StreamDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamDirection::Downstream => write!(f, "downstream"),
            StreamDirection::Upstream => write!(f, "upstream"),
        }
    }
}

impl Toxic {
    /// Get the toxic name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Sets a name to this Toxic if the name is an empty string.
    /// The default name format is {type}_{direction}
    pub fn set_default_name(&mut self) {
        if self.name.is_empty() {
            self.name = format!("{}_{}", self.kind.get_name(), self.direction);
        }
    }
}

impl ToxicEvent {
    /// Create a new toxic event
    pub fn new(proxy_name: String, kind: ToxicEventKind) -> Self {
        ToxicEvent { proxy_name, kind }
    }
}

impl ToxicKind {
    pub(crate) fn has_close_logic(&self) -> bool {
        matches!(
            self,
            ToxicKind::SlowClose { .. } | ToxicKind::LimitData { .. }
        )
    }

    pub(crate) fn is_stateful(&self) -> bool {
        matches!(self, ToxicKind::LimitData { .. })
    }

    pub(crate) fn chunk_buffer_capacity(&self) -> usize {
        match self {
            ToxicKind::Latency { .. } => 1024,
            _ => 1,
        }
    }

    /// Returns the URL-safe name for the toxic kind
    pub fn get_name(&self) -> &'static str {
        match self {
            ToxicKind::Noop => "noop",
            ToxicKind::Latency { .. } => "latency",
            ToxicKind::Timeout { .. } => "timeout",
            ToxicKind::Bandwidth { .. } => "bandwidth",
            ToxicKind::SlowClose { .. } => "slow_close",
            ToxicKind::Slicer { .. } => "slicer",
            ToxicKind::LimitData { .. } => "limit_data",
        }
    }
}

pub(super) fn update_toxic_list_in_place(
    toxics: &mut Vec<Toxic>,
    event_kind: ToxicEventKind,
    direction: StreamDirection,
) -> Result<(), ToxicEventKind> {
    match event_kind {
        ToxicEventKind::AddToxic(toxic) => {
            if toxic.direction == direction {
                toxics.push(toxic);
            } else {
                return Err(ToxicEventKind::AddToxic(toxic));
            }
        }
        ToxicEventKind::UpdateToxic(toxic) => {
            let old_toxic = if toxic.direction == direction {
                toxics
                    .iter_mut()
                    .find(|el| el.get_name() == toxic.get_name())
            } else {
                None
            };
            if let Some(old_toxic) = old_toxic {
                let _ = mem::replace(old_toxic, toxic);
            } else {
                return Err(ToxicEventKind::UpdateToxic(toxic));
            }
        }
        ToxicEventKind::RemoveToxic(toxic_name) => {
            let index = toxics
                .iter()
                .position(|el| el.get_name() == toxic_name)
                .ok_or(ToxicEventKind::RemoveToxic(toxic_name))?;
            toxics.remove(index);
        }
        ToxicEventKind::RemoveAllToxics => {
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
            ToxicKind::Latency { latency, jitter } => {
                write!(f, "Latency({}, {})", latency, jitter)
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
            ToxicKind::Slicer {
                average_size,
                size_variation,
                delay,
            } => {
                write!(f, "Slicer({}, {}, {})", average_size, size_variation, delay)
            }
            ToxicKind::LimitData { bytes } => {
                write!(f, "LimitData({})", bytes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string, Error as SerdeError};

    #[test]
    fn test_display_noop() {
        let toxic = Toxic {
            kind: ToxicKind::Noop,
            name: "boo".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "boo: Noop";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_latency() {
        let toxic = Toxic {
            kind: ToxicKind::Latency {
                latency: 49,
                jitter: 5,
            },
            name: "t2".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t2: Latency(49, 5)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_timeout() {
        let toxic = Toxic {
            kind: ToxicKind::Timeout { timeout: 2000 },
            name: "t3".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t3: Timeout(2000)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_bandwidth() {
        let toxic = Toxic {
            kind: ToxicKind::Bandwidth { rate: 2345 },
            name: "t4".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t4: Bandwidth(2345)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_slicer() {
        let toxic = Toxic {
            kind: ToxicKind::Slicer {
                average_size: 128,
                size_variation: 64,
                delay: 100,
            },
            name: "t5".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t5: Slicer(128, 64, 100)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_slow_close() {
        let toxic = Toxic {
            kind: ToxicKind::SlowClose { delay: 1200 },
            name: "t6".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t6: SlowClose(1200)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_display_limit_data() {
        let toxic = Toxic {
            kind: ToxicKind::LimitData { bytes: 64500 },
            name: "t7".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };
        let expected = "t7: LimitData(64500)";
        assert_eq!(expected, toxic.to_string());
    }

    #[test]
    fn test_noop_serde() {
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
    fn test_noop_ser_without_toxicity() {
        let input = "{\"type\":\"noop\",\"name\":\"foo\",\"direction\":\"upstream\"}";
        let expected = Toxic {
            kind: ToxicKind::Noop,
            name: "foo".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Upstream,
        };

        let deserialized = from_str(&input).unwrap();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_noop_ser_without_direction() {
        let input = "{\"type\":\"noop\",\"name\":\"foo\",\"toxicity\":0.55}";
        let expected = Toxic {
            kind: ToxicKind::Noop,
            name: "foo".to_owned(),
            toxicity: 0.55,
            direction: StreamDirection::Downstream,
        };

        let deserialized = from_str(&input).unwrap();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_latency_serde() {
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

    #[test]
    fn test_noop_de_without_name() {
        let input = "{\"type\":\"noop\"}";
        let expected = Toxic {
            kind: ToxicKind::Noop,
            name: "noop_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_latency_de_without_name() {
        let input = "{\"type\":\"latency\",\"attributes\":{\"latency\":4321,\"jitter\":5}}";
        let expected = Toxic {
            kind: ToxicKind::Latency {
                latency: 4321,
                jitter: 5,
            },
            name: "latency_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_timeout_de_without_name() {
        let input = "{\"type\":\"timeout\",\"attributes\":{\"timeout\":2000}}";
        let expected = Toxic {
            kind: ToxicKind::Timeout { timeout: 2000 },
            name: "timeout_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_bandwidth_de_without_name() {
        let input = "{\"type\":\"bandwidth\",\"attributes\":{\"rate\":500}}";
        let expected = Toxic {
            kind: ToxicKind::Bandwidth { rate: 500 },
            name: "bandwidth_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_slow_close_de_without_name() {
        let input = "{\"type\":\"slow_close\",\"attributes\":{\"delay\":3000}}";
        let expected = Toxic {
            kind: ToxicKind::SlowClose { delay: 3000 },
            name: "slow_close_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_slicer_de_without_name() {
        let input = "{\"type\":\"slicer\",\"attributes\":{\"average_size\":100,\"size_variation\": 9,\"delay\": 50}}";
        let expected = Toxic {
            kind: ToxicKind::Slicer {
                average_size: 100,
                size_variation: 9,
                delay: 50,
            },
            name: "slicer_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }

    #[test]
    fn test_limit_data_de_without_name() {
        let input = "{\"type\":\"limit_data\",\"attributes\":{\"bytes\":1024}}";
        let expected = Toxic {
            kind: ToxicKind::LimitData { bytes: 1024 },
            name: "limit_data_downstream".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        };

        let mut deserialized: Toxic = from_str(&input).unwrap();
        assert_eq!("", &deserialized.name);
        deserialized.set_default_name();
        assert_eq!(expected, deserialized);
    }
}
