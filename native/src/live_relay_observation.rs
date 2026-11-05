//! 远程语义观察合同；不依赖 UIA/窗口，认证与 session/epoch 由外层控制 owner 校验。
use crate::live_relay_viewer_identity::{identifier, parse};
use serde::{Deserialize, Serialize};

pub const MAX_OBSERVATIONS: usize = 256;
const MAX_VALUE: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Invalid,
    Limit,
    Sequence,
    Duplicate,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Unknown,
    Detected,
    Observing,
    Stable,
    Triggered,
    Stale,
    Error,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    UiAutomation,
    AppAdapter,
    Vision,
    Unknown,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Exact,
    High,
    Medium,
    Low,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Locator {
    pub automation_id: Option<String>,
    pub name: Option<String>,
    pub control_type: String,
    pub ancestor_path: Vec<String>,
    pub runtime_id: Option<Vec<i32>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub sequence: u64,
    pub state: State,
    pub source: Source,
    pub confidence: Confidence,
    pub observed_at_ms: u64,
    pub stable_since_ms: Option<u64>,
    pub locator: Option<Locator>,
    pub value: Option<serde_json::Value>,
    pub reason: Option<String>,
}
fn text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.chars().count() <= max
}
impl Observation {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let value: Self = parse(bytes).map_err(|_| Error::Json)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if !identifier(&self.observation_id)
            || self.sequence == 0
            || self.observed_at_ms == 0
            || self
                .stable_since_ms
                .is_some_and(|at| at > self.observed_at_ms)
            || (matches!(self.state, State::Unknown | State::Stale | State::Error)
                && self.value.is_some())
            || (self.source == Source::Unknown && self.confidence != Confidence::Low)
            || (self.source == Source::Vision && self.confidence == Confidence::Exact)
            || (self.source == Source::UiAutomation && self.locator.is_none())
            || self.reason.as_ref().is_some_and(|v| !text(v, 512))
        {
            return Err(Error::Invalid);
        }
        if let Some(locator) = &self.locator {
            if !text(&locator.control_type, 160)
                || locator
                    .automation_id
                    .as_ref()
                    .is_some_and(|v| !text(v, 512))
                || locator.name.as_ref().is_some_and(|v| !text(v, 512))
                || locator.ancestor_path.len() > 64
                || locator.ancestor_path.iter().any(|v| !text(v, 512))
                || locator.runtime_id.as_ref().is_some_and(|v| v.len() > 64)
            {
                return Err(Error::Invalid);
            }
        }
        if let Some(value) = &self.value {
            if serde_json::to_vec(value).map_err(|_| Error::Json)?.len() > MAX_VALUE {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
}

#[path = "live_relay_observation_store.rs"]
mod store;
pub use store::Observations;
#[path = "live_relay_observation_summary.rs"]
mod summary;
pub use summary::{Capabilities, Capability, Summary};
#[cfg(test)]
#[path = "live_relay_observation_tests.rs"]
mod tests;
