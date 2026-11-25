//! 能力报告与观察汇总；能力缺失不是空报告，汇总不替代具体观察的安全校验。
use super::{Error, Observations, State};
use crate::live_relay_viewer_identity::parse;
use serde::Deserialize;
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    UiaTree,
    Invoke,
    RangeValue,
    Value,
    Toggle,
    Scroll,
    Adapter,
    Vision,
}
#[derive(Clone)]
pub struct Capabilities(Vec<Capability>);
#[derive(Deserialize)]
struct Snapshot {
    session: Session,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    observation_capabilities: Vec<Capability>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Summary<'a> {
    Unsupported,
    Error(Option<&'a str>),
    Observing,
    Stable,
    Stale,
}
impl Capabilities {
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, Error> {
        let snapshot: Snapshot = parse(bytes).map_err(|_| Error::Json)?;
        let values = snapshot.session.observation_capabilities;
        if values.len() > 8 {
            return Err(Error::Limit);
        }
        for (i, value) in values.iter().enumerate() {
            if values[..i].contains(value) {
                return Err(Error::Duplicate);
            }
        }
        Ok(Self(values))
    }
    pub fn values(&self) -> &[Capability] {
        &self.0
    }
    pub fn summarize<'a>(&self, observations: &'a Observations) -> Summary<'a> {
        if self.0.is_empty() {
            return Summary::Unsupported;
        }
        if let Some(value) = observations.iter().find(|v| v.state == State::Error) {
            return Summary::Error(value.reason.as_deref());
        }
        if observations
            .iter()
            .any(|v| matches!(v.state, State::Detected | State::Observing))
        {
            return Summary::Observing;
        }
        if observations
            .iter()
            .any(|v| matches!(v.state, State::Stable | State::Triggered))
        {
            return Summary::Stable;
        }
        if observations.iter().next().is_some() {
            Summary::Stale
        } else {
            Summary::Observing
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn capabilities_require_explicit_unique_known_report() {
        for input in [
            json!({}),
            json!({"session":{}}),
            json!({"session":{"observationCapabilities":["invented"]}}),
            json!({"session":{"observationCapabilities":["value","value"]}}),
        ] {
            assert!(Capabilities::from_snapshot(&serde_json::to_vec(&input).unwrap()).is_err());
        }
        let empty =
            Capabilities::from_snapshot(br#"{"session":{"observationCapabilities":[]}}"#).unwrap();
        assert_eq!(
            empty.summarize(&Observations::default()),
            Summary::Unsupported
        );
    }
    #[test]
    fn summary_preserves_error_observing_stable_stale_priority() {
        let caps =
            Capabilities::from_snapshot(br#"{"session":{"observationCapabilities":["adapter"]}}"#)
                .unwrap();
        assert_eq!(caps.summarize(&Observations::default()), Summary::Observing);
        let values = ["stale", "stable", "observing", "error"];
        for count in 1..=4 {
            let observations: Vec<_> = values[..count].iter().enumerate().map(|(i,state)| json!({
                "observationId":format!("id:{i}"),"sequence":1,"state":state,"source":"app_adapter",
                "confidence":"high","observedAtMs":1,"reason":"reason"
            })).collect();
            let state = Observations::from_snapshot(
                &serde_json::to_vec(&json!({"observations":observations})).unwrap(),
            )
            .unwrap();
            let expected = match count {
                1 => Summary::Stale,
                2 => Summary::Stable,
                3 => Summary::Observing,
                _ => Summary::Error(Some("reason")),
            };
            assert_eq!(caps.summarize(&state), expected);
        }
    }
}
