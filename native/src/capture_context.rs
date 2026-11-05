//! 当前采集的来源坐标与工作负载；来源矩形不是编辑后的输出大小。
use crate::session_capture::{Axis, Kind};
pub(crate) struct Context {
    pub kind: Kind,
    pub rect: Option<crate::session_window::Rect>,
    pub axis: Option<Axis>,
    pub backend: &'static str,
    pub recording: Option<crate::session_capture::Recording>,
}
impl Context {
    pub fn region(
        origin: Option<windows::Win32::Foundation::POINT>,
        width: u32,
        height: u32,
        backend: &'static str,
    ) -> Self {
        Self {
            kind: Kind::Region,
            rect: origin.map(|origin| crate::session_window::Rect {
                x: origin.x as f64,
                y: origin.y as f64,
                w: width as f64,
                h: height as f64,
            }),
            axis: None,
            backend,
            recording: None,
        }
    }
    pub fn metadata(&self, hdr: bool, downgraded: bool) -> serde_json::Value {
        let mut value = serde_json::json!({
            "kind": if self.kind == Kind::Long { "long" } else { "region" },
            "dynamicRange": if hdr { "hdr" } else { "sdr" },
            "bitDepth": if hdr { 16 } else { 8 },
            "colorSpace": if hdr { "bt2020-pq" } else { "srgb" },
            "captureBackend": self.backend, "downgradedFromHdr": downgraded,
        });
        if let Some(rect) = self.rect.filter(|rect| rect.valid()) {
            value["sourceRect"] = serde_json::json!({"x":rect.x,"y":rect.y,"w":rect.w,"h":rect.h});
            value["otterNativeSourceCoordinates"] = "physical".into();
        }
        if let Some(axis) = self.axis {
            value["scrollAxis"] = if axis == Axis::Horizontal {
                "horizontal"
            } else {
                "vertical"
            }
            .into();
        }
        if let Some(recording) = &self.recording {
            value["otterNativeRecording"] = serde_json::json!(recording);
        }
        value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::POINT;
    #[test]
    fn recorded_single_frame_does_not_invent_axis_and_roundtrips_details() {
        let mut context = Context::region(Some(POINT { x: -10, y: 20 }), 100, 80, "gdi");
        context.kind = Kind::Long;
        context.recording = Some(crate::session_capture::Recording {
            frame_count: 1,
            direction: None,
            spatial_position: None,
        });
        let value = context.metadata(false, false);
        assert!(value.get("scrollAxis").is_none());
        let encoded = value.to_string();
        let decoded = crate::session_capture::decode(&encoded).unwrap();
        assert_eq!(decoded.recording.unwrap(), context.recording.unwrap());
        assert_eq!(decoded.source_rect.unwrap().x, -10.0);
    }
    #[test]
    fn region_metadata_preserves_negative_physical_rectangle_and_backend() {
        let context = Context::region(Some(POINT { x: -1920, y: -100 }), 400, 300, "wgc");
        let value = context.metadata(true, false);
        assert_eq!(value["sourceRect"]["x"], -1920.0);
        assert_eq!(value["otterNativeSourceCoordinates"], "physical");
        assert_eq!(value["captureBackend"], "wgc");
        assert_eq!(value["kind"], "region");
        assert!(crate::session_capture::decode(&value.to_string()).is_ok());
    }
    #[test]
    fn long_capture_records_viewport_and_direction_without_inventing_missing_coordinates() {
        let mut context = Context::region(Some(POINT { x: 10, y: 20 }), 100, 80, "gdi");
        context.kind = Kind::Long;
        context.axis = Some(Axis::Horizontal);
        let value = context.metadata(false, false);
        assert_eq!(value["kind"], "long");
        assert_eq!(value["scrollAxis"], "horizontal");
        assert_eq!(value["sourceRect"]["w"], 100.0);
        assert!(crate::session_capture::decode(&value.to_string()).is_ok());
        let missing = Context::region(None, 100, 80, "gdi").metadata(false, false);
        assert!(missing.get("sourceRect").is_none());
        assert!(missing.get("otterNativeSourceCoordinates").is_none());
    }
}
