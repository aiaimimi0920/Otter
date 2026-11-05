//! Wall v1 的有界不可变快照；图片/raw/PNG已实现，不伪造Art或输入附件。
use serde::{Deserialize, Serialize};
#[path = "tile_wire_validation.rs"]
mod validation;
pub const PROTOCOL: &str = "loom.wall.v1";
pub const MAX_PIXELS: u64 = 16_777_216;
pub const MAX_SAFE: u64 = 9_007_199_254_740_991;
pub type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    pub protocol_version: String,
    pub endpoint_id: String,
    pub device_id: String,
    pub output_id: String,
    pub pixel_size: Size,
    pub render_modes: Vec<String>,
    pub input_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<Display>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduled_presentation: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Display {
    pub name: String,
    pub can_identify: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn contains(&self, other: &Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }
    pub fn intersects(&self, b: &Self) -> bool {
        self.x < b.x + b.width
            && b.x < self.x + self.width
            && self.y < b.y + b.height
            && b.y < self.y + self.height
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Rotation {
    #[serde(rename = "deg0")]
    Zero,
    #[serde(rename = "deg90")]
    Right,
    #[serde(rename = "deg180")]
    Half,
    #[serde(rename = "deg270")]
    Left,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tile {
    pub tile_id: String,
    pub endpoint_id: String,
    pub rect: Rect,
    pub rotation: Rotation,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub kind: String,
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Placement {
    pub placement_id: String,
    pub source: Source,
    pub rect: Rect,
    pub source_crop: Rect,
    pub z_index: i32,
    pub interactive: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layout {
    pub protocol_version: String,
    pub wall_id: String,
    pub revision: u64,
    pub bounds: Rect,
    pub tiles: Vec<Tile>,
    pub placements: Vec<Placement>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub protocol_version: String,
    pub revision: u64,
    pub endpoints: Vec<EndpointStatus>,
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub presentations: Vec<Presentation>,
    pub timing: Option<Timing>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EndpointStatus {
    pub endpoint: Endpoint,
    pub online: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub applied_revision: Option<u64>,
    pub presentation: Option<PresentationReport>,
    pub identification: Option<Identification>,
    pub scene: Option<SceneReport>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identification {
    pub request_id: String,
    pub remaining_ms: u64,
    pub applied: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Presentation {
    pub wall_id: String,
    pub revision: u64,
    pub mode: Mode,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Frozen,
    Black,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationReport {
    pub revision: u64,
    pub outcome: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timing {
    pub clock_id: String,
    pub server_time_ms: u64,
    pub scenes: Vec<Scene>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scene {
    pub wall_id: String,
    pub revision: u64,
    pub prepared_at_ms: u64,
    pub activate_at_ms: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneReport {
    pub revision: u64,
    pub prepared: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub applied_at_ms: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub clock_uncertainty_ms: Option<u64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresenterLease {
    pub protocol_version: String,
    pub lease_id: String,
    pub lease_ttl_ms: u64,
}
pub fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:/-".contains(&b))
}
fn required_nullable<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}
pub fn image_id(s: &str) -> bool {
    s.len() == 71
        && s.starts_with("sha256:")
        && s.as_bytes()[7..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}
impl State {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 10 * 1024 * 1024 {
            return Err("tile_state_limit");
        }
        let state: Self = serde_json::from_slice(bytes).map_err(|_| "tile_state_invalid")?;
        validation::state(&state)?;
        Ok(state)
    }
}
impl Endpoint {
    pub fn image(device: &str, output: &crate::tile_outputs::TileOutput) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            protocol_version: PROTOCOL.into(),
            endpoint_id: format!(
                "tile-{:x}",
                Sha256::digest(format!("{device}\n{}", output.output_id))
            ),
            device_id: device.into(),
            output_id: output.output_id.clone(),
            pixel_size: Size {
                width: output.width,
                height: output.height,
            },
            render_modes: vec!["image".into(), "raw_bgra".into(), "surface_v1".into()],
            input_capabilities: vec!["pointer".into()],
            display: Some(Display {
                name: output.name.clone(),
                can_identify: true,
            }),
            scheduled_presentation: Some(true),
        }
    }
}
