//! 主干 captureMeta 的有界只读合同；缺失字段不猜测，未知扩展仍由原始会话 JSON 保留。
use serde::Deserialize;
use std::borrow::Cow;
#[path = "session_capture_long.rs"]
mod long;
pub use long::{Direction, Recording};
pub const MAX_BYTES: usize = 4096;
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Region,
    Long,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Axis {
    Vertical,
    Horizontal,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Range {
    Sdr,
    Hdr,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum Color {
    #[serde(rename = "srgb")]
    Srgb,
    #[serde(rename = "bt2020-pq")]
    Bt2020Pq,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata<'a> {
    #[serde(rename = "otterNativeRecording")]
    pub recording: Option<Recording>,
    pub kind: Option<Kind>,
    pub source_rect: Option<crate::session_window::Rect>,
    pub scroll_axis: Option<Axis>,
    pub dynamic_range: Option<Range>,
    pub bit_depth: Option<u8>,
    pub color_space: Option<Color>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub capture_backend: Option<Cow<'a, str>>,
    pub downgraded_from_hdr: Option<bool>,
}
#[derive(Debug)]
pub enum Error {
    Capacity,
    Json(serde_json::Error),
    Invalid,
}
pub fn decode(raw: &str) -> Result<Metadata<'_>, Error> {
    if raw.len() > MAX_BYTES {
        return Err(Error::Capacity);
    }
    let value: Metadata<'_> = serde_json::from_str(raw).map_err(Error::Json)?;
    if value.source_rect.is_some_and(|rect| !rect.valid())
        || value
            .recording
            .as_ref()
            .is_some_and(|recording| !recording.valid(value.kind, value.scroll_axis))
        || value
            .bit_depth
            .is_some_and(|depth| !matches!(depth, 8 | 16))
        || value
            .capture_backend
            .as_ref()
            .is_some_and(|backend| backend.len() > 256 || backend.chars().any(char::is_control))
        || matches!(
            (value.dynamic_range, value.bit_depth),
            (Some(Range::Hdr), Some(8)) | (Some(Range::Sdr), Some(16))
        )
        || matches!(
            (value.dynamic_range, value.color_space),
            (Some(Range::Hdr), Some(Color::Srgb)) | (Some(Range::Sdr), Some(Color::Bt2020Pq))
        )
        || (value.dynamic_range == Some(Range::Hdr) && value.downgraded_from_hdr == Some(true))
    {
        return Err(Error::Invalid);
    }
    Ok(value)
}
#[cfg(test)]
#[path = "session_capture_tests.rs"]
mod tests;
