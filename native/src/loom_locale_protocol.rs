//! Bounded display-language wire values; remote requests never carry arbitrary settings or paths.
use crate::display_language::DisplayLanguage;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const MAX_REVISION: u64 = 9_007_199_254_740_991;
pub const MAX_JSON: usize = 16 * 1024;
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub request_id: String,
    pub expected_revision: u64,
    pub language: String,
    pub receiver_generation: String,
    pub expires_at: u64,
    pub valid_for_ms: u64,
}

impl Request {
    pub fn language(&self) -> Option<DisplayLanguage> {
        DisplayLanguage::parse(&self.language)
    }

    pub fn deadline(&self, started: Instant) -> Option<Instant> {
        started.checked_add(Duration::from_millis(self.valid_for_ms))
    }

    fn valid(&self, generation: &str) -> bool {
        identifier(&self.request_id)
            && self.expected_revision <= MAX_REVISION
            && self.language().is_some()
            && self.receiver_generation == generation
            && (1..=15_000).contains(&self.valid_for_ms)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub request_id: String,
    pub status: &'static str,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_revision: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report<'a> {
    pub receiver_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_generation: Option<&'a str>,
    pub language: &'a str,
    pub revision: u64,
    pub supported_languages: [&'a str; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<&'a Receipt>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    loom_id: String,
    device_id: String,
    locale: Snapshot,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub receiver_generation: String,
    pub language: String,
    pub revision: u64,
    pub supported_languages: Vec<String>,
    pub active: bool,
    pub request: Option<Request>,
}

impl Snapshot {
    pub fn parse(
        bytes: &[u8],
        loom: &str,
        device: &str,
        expected_generation: Option<&str>,
    ) -> Result<Self, &'static str> {
        if bytes.len() > MAX_JSON {
            return Err("locale_response_limit");
        }
        let value: Envelope =
            serde_json::from_slice(bytes).map_err(|_| "locale_response_invalid")?;
        let snapshot = value.locale;
        if value.loom_id != loom
            || value.device_id != device
            || !identifier(&snapshot.receiver_generation)
            || expected_generation.is_some_and(|expected| expected != snapshot.receiver_generation)
            || !snapshot.active
            || DisplayLanguage::parse(&snapshot.language).is_none()
            || snapshot.revision > MAX_REVISION
            || snapshot.supported_languages.len() != 2
            || !snapshot.supported_languages.iter().any(|v| v == "en")
            || !snapshot.supported_languages.iter().any(|v| v == "zh-Hans")
            || snapshot
                .request
                .as_ref()
                .is_some_and(|r| !r.valid(&snapshot.receiver_generation))
        {
            return Err("locale_response_identity");
        }
        Ok(snapshot)
    }
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}

#[cfg(test)]
#[path = "loom_locale_protocol_tests.rs"]
mod tests;
