//! 对齐主干美化预设与布局；未知背景回退默认日落，输出尺寸仍受像素预算限制。
use super::*;
pub(in crate::desktop) struct Settings {
    pub padding: u32,
    pub radius: f64,
    pub shadow: bool,
    pub shadow_sigma: f32,
    pub shadow_offset: u32,
    pub colors: ([u8; 3], [u8; 3]),
}
impl Settings {
    pub(in crate::desktop) fn read(edit: &serde_json::Value) -> Result<Option<Self>> {
        let value = &edit["beautify"];
        if value["enabled"].as_bool() != Some(true) {
            return Ok(None);
        }
        let raw_padding = value["padding"].as_f64().unwrap_or(0.0);
        let padding = raw_padding.max(0.0).round();
        if !padding.is_finite() || padding > 8192.0 {
            return Err(Error::new(
                E_INVALIDARG,
                "Beautify padding exceeds output limits",
            ));
        }
        let colors = match value["backgroundId"].as_str().unwrap_or("sunset") {
            "ocean" => ([33, 147, 176], [109, 213, 237]),
            "violet" => ([142, 45, 226], [74, 0, 224]),
            "mint" => ([17, 153, 142], [56, 239, 125]),
            "slate" => ([35, 37, 38], [65, 67, 69]),
            "white" => ([255; 3], [255; 3]),
            "graphite" => ([31, 41, 51], [31, 41, 51]),
            _ => ([255, 126, 95], [254, 180, 123]),
        };
        Ok(Some(Self {
            padding: padding as u32,
            radius: value["cornerRadius"].as_f64().unwrap_or(0.0).max(0.0),
            shadow: value["shadow"].as_bool().unwrap_or(false),
            shadow_sigma: ((raw_padding / 2.0).round().max(8.0) / 2.0) as f32,
            shadow_offset: (raw_padding / 4.0).round().max(4.0) as u32,
            colors,
        }))
    }
}
