//! Device-bound snapshots. Neither the manager nor the receiver may supply executable actions.
use crate::{
    loom_locale_protocol::{identifier, Receipt},
    shortcut_policy::{Policy, MAX_REVISION},
};
use serde::{Deserialize, Serialize};
pub const PROTOCOL: &str = "neuro.otter.shortcuts.v1";
pub const MAX_JSON: usize = 16 * 1024;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub protocol: &'static str,
    pub receiver_id: String,
    pub receiver_generation: Option<String>,
    pub revision: u64,
    pub policy: Policy,
    pub enabled: bool,
    pub suppressed: bool,
    pub registered: [bool; 6],
    pub result: Option<Receipt>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub expected_revision: u64,
    pub receiver_generation: String,
    pub expires_at: u64,
    pub valid_for_ms: u64,
    pub policy: Policy,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    protocol: String,
    loom_id: String,
    device_id: String,
    pub receiver_generation: String,
    pub request: Option<Request>,
}
impl Snapshot {
    pub fn parse(
        bytes: &[u8],
        loom: &str,
        device: &str,
        generation: Option<&str>,
        now: u64,
    ) -> Result<Self, &'static str> {
        if bytes.len() > MAX_JSON {
            return Err("shortcut_response_limit");
        }
        let s: Self = serde_json::from_slice(bytes).map_err(|_| "shortcut_response_invalid")?;
        if s.protocol != PROTOCOL
            || s.loom_id != loom
            || s.device_id != device
            || !identifier(&s.receiver_generation)
            || generation.is_some_and(|g| g != s.receiver_generation)
        {
            return Err("shortcut_response_identity");
        }
        if let Some(r) = &s.request {
            if !identifier(&r.request_id)
                || r.receiver_generation != s.receiver_generation
                || r.expected_revision >= MAX_REVISION
                || !(1..=15_000).contains(&r.valid_for_ms)
                || r.expires_at <= now
                || r.expires_at > now.saturating_add(15_000)
            {
                return Err("shortcut_request_invalid");
            }
            r.policy.validate()?;
        }
        Ok(s)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn report_cannot_deliver_foreign_stale_or_oversized_intent() {
        let mut v = serde_json::json!({"protocol":PROTOCOL,"loomId":"a","deviceId":"b","receiverGeneration":"gen","request":{"requestId":"r","receiverGeneration":"gen","expectedRevision":0,"expiresAt":12000,"validForMs":1000,"policy":Policy::default()}});
        let parse = |v: &serde_json::Value| {
            Snapshot::parse(
                &serde_json::to_vec(v).unwrap(),
                "a",
                "b",
                Some("gen"),
                11000,
            )
        };
        assert!(parse(&v).is_ok());
        for (field, value) in [
            ("deviceId", "other"),
            ("loomId", "elsewhere"),
            ("receiverGeneration", "old"),
        ] {
            let mut bad = v.clone();
            bad[field] = value.into();
            assert!(parse(&bad).is_err());
        }
        v["request"]["expiresAt"] = 11000.into();
        assert!(parse(&v).is_err());
        assert!(Snapshot::parse(&vec![0; MAX_JSON + 1], "a", "b", None, 0).is_err());
    }
}
