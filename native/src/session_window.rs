//! 主干窗口恢复元数据；只验证候选，不访问显示器或创建 HWND。
use serde::Deserialize;
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}
impl Offset {
    fn valid(self) -> bool {
        [self.x, self.y]
            .iter()
            .all(|value| value.is_finite() && value.abs() <= i32::MAX as f64)
    }
}
impl Rect {
    pub fn valid(&self) -> bool {
        [self.x, self.y, self.w, self.h]
            .iter()
            .all(|value| value.is_finite())
            && self.w > 0.0
            && self.h > 0.0
            && [self.x, self.y, self.x + self.w, self.y + self.h]
                .iter()
                .all(|value| *value >= i32::MIN as f64 && *value <= i32::MAX as f64)
            && self.w <= i32::MAX as f64
            && self.h <= i32::MAX as f64
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window<'a> {
    pub id: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(rename = "type", default)]
    pub node_type: Option<String>,
    #[serde(default)]
    pub art_id: Option<String>,
    #[serde(default)]
    pub otter_native_local_art: bool,
    #[serde(default, borrow)]
    pub otter_native_local_art_ports: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow, deserialize_with = "present_projection_edit")]
    pub port_visibility: Option<&'a serde_json::value::RawValue>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub minified: Option<bool>,
    #[serde(default)]
    pub saved_rect: Option<Rect>,
    #[serde(default)]
    pub crop_offset: Option<Offset>,
    #[serde(default)]
    pub opacity_normal: Option<f64>,
    #[serde(default)]
    pub opacity_mini: Option<f64>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub src: Option<std::borrow::Cow<'a, str>>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub preview_src: Option<std::borrow::Cow<'a, str>>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub rasterized_annotation_layer_src: Option<std::borrow::Cow<'a, str>>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub file_path: Option<std::borrow::Cow<'a, str>>,
    #[serde(default, borrow)]
    pub params: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub outputs: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub execution_config: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub image_edit_state: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub sticker_edit_propagation: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub annotation_state: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow, deserialize_with = "present_projection_edit")]
    pub otter_native_projection_edit: Option<&'a serde_json::value::RawValue>,
    #[serde(default, borrow)]
    pub capture_meta: Option<&'a serde_json::value::RawValue>,
    #[serde(
        default,
        borrow,
        deserialize_with = "crate::session_assets::optional_text"
    )]
    pub otter_native_capture_original_src: Option<std::borrow::Cow<'a, str>>,
}
// 缺字段兼容旧普通工作区；显式null仍传给记录校验，不能静默丢掉协作身份。
fn present_projection_edit<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<&'de serde_json::value::RawValue>, D::Error> {
    <&serde_json::value::RawValue>::deserialize(decoder).map(Some)
}
impl Window<'_> {
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
    pub fn is_art(&self) -> bool {
        self.node_type.as_deref() == Some("art")
    }
    pub fn is_minified(&self) -> bool {
        self.minified.unwrap_or(false)
    }
    pub fn opacity(&self, minified: bool) -> f64 {
        if minified {
            self.opacity_mini.unwrap_or(0.9)
        } else {
            self.opacity_normal.unwrap_or(1.0)
        }
    }
    pub fn valid(&self) -> bool {
        self.rect().valid()
            && self.capture_metadata().is_ok()
            && self
                .otter_native_capture_original_src
                .as_deref()
                .is_none_or(|source| {
                    source.len() <= crate::session_document::MAX_SESSION_BYTES
                        && source.starts_with("data:image/png;base64,")
                        && self
                            .capture_metadata()
                            .ok()
                            .flatten()
                            .is_some_and(|metadata| {
                                metadata.dynamic_range == Some(crate::session_capture::Range::Hdr)
                            })
                })
            && self.saved_rect.is_none_or(|rect| rect.valid())
            && self.crop_offset.is_none_or(Offset::valid)
            && self.expanded_rect().is_some()
            && [self.opacity(false), self.opacity(true)]
                .iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
            && self
                .art_id
                .as_ref()
                .is_none_or(|id| id.len() <= 256 && !id.chars().any(char::is_control))
            && self
                .node_type
                .as_ref()
                .is_none_or(|kind| kind.len() <= 256 && !kind.chars().any(char::is_control))
    }
    /// 仅解析来源元数据；不授权 filePath、资源加载或 HDR 原图导出。
    pub fn capture_metadata(
        &self,
    ) -> Result<Option<crate::session_capture::Metadata<'_>>, crate::session_capture::Error> {
        self.capture_meta
            .map(|value| crate::session_capture::decode(value.get()))
            .transpose()
    }
    /// 对齐主干 computeRestoredMinifiedStickerWindow；移动后的缩略窗口保留当前位置。
    /// 缺失 savedRect 时不猜测展开尺寸，保持当前框；不在读取阶段自动移动 HWND。
    pub fn expanded_rect(&self) -> Option<Rect> {
        expanded_rect(
            self.rect(),
            self.is_minified(),
            self.saved_rect,
            self.crop_offset,
        )
    }
}
pub fn expanded_rect(
    current: Rect,
    minified: bool,
    saved: Option<Rect>,
    offset: Option<Offset>,
) -> Option<Rect> {
    let result = if minified {
        saved
            .map(|saved| match offset {
                Some(offset) => Rect {
                    x: current.x - offset.x,
                    y: current.y - offset.y,
                    ..saved
                },
                None => saved,
            })
            .unwrap_or(current)
    } else {
        current
    };
    result.valid().then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window() -> Window<'static> {
        serde_json::from_str(r#"{"id":"s","x":-1920.5,"y":-100.25,"w":800.5,"h":600.25}"#).unwrap()
    }
    #[test]
    fn missing_defaults_and_explicit_zero_match_main_mapping() {
        let mut value = window();
        assert!(value.valid());
        assert!(!value.is_art() && !value.is_minified());
        assert_eq!((value.opacity(false), value.opacity(true)), (1.0, 0.9));
        value.opacity_normal = Some(0.0);
        value.opacity_mini = Some(0.0);
        assert!(value.valid());
        assert_eq!((value.opacity(false), value.opacity(true)), (0.0, 0.0));
        value.node_type = Some("art".into());
        value.minified = Some(true);
        assert!(value.is_art() && value.is_minified());
        value.saved_rect = Some(value.rect());
        assert!(value.valid());
    }
    #[test]
    fn invalid_geometry_and_opacity_are_rejected_before_hwnd_conversion() {
        for bad in [f64::NAN, f64::INFINITY, 0.0, -1.0, i32::MAX as f64 + 1.0] {
            let mut value = window();
            value.w = bad;
            assert!(!value.valid());
        }
        let mut value = window();
        value.x = i32::MAX as f64;
        assert!(!value.valid());
        for bad in [-0.1, 1.1, f64::NAN] {
            let mut value = window();
            value.opacity_mini = Some(bad);
            assert!(!value.valid());
        }
    }
    #[test]
    fn moved_minified_window_expands_relative_to_current_position() {
        let json = r#"{"id":"s","x":-800,"y":50,"w":100,"h":100,"minified":true,
            "savedRect":{"x":-1200,"y":-200,"w":800,"h":600},"cropOffset":{"x":150,"y":75}}"#;
        let mut value: Window<'_> = serde_json::from_str(json).unwrap();
        assert!(value.valid());
        assert_eq!(
            value.expanded_rect(),
            Some(Rect {
                x: -950.0,
                y: -25.0,
                w: 800.0,
                h: 600.0
            })
        );
        value.crop_offset = None;
        assert_eq!(value.expanded_rect(), value.saved_rect);
        value.minified = Some(false);
        assert_eq!(value.expanded_rect(), Some(value.rect()));
        value.minified = Some(true);
        value.saved_rect = None;
        assert_eq!(value.expanded_rect(), Some(value.rect()));
    }
    #[test]
    fn minified_offset_does_not_overflow_native_window_coordinates() {
        let mut value = window();
        value.minified = Some(true);
        value.saved_rect = Some(value.rect());
        value.crop_offset = Some(Offset {
            x: i32::MAX as f64,
            y: 0.0,
        });
        assert!(!value.valid());
        value.crop_offset = Some(Offset {
            x: f64::NAN,
            y: 0.0,
        });
        assert!(!value.valid());
        value.crop_offset = Some(Offset { x: -25.0, y: -10.0 });
        assert!(value.valid());
    }
}
