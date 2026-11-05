//! 采集来源只绑定当前文档身份；路径是来源记录，不是文件读写/删除授权或当前编辑结果。
use super::Canvas;
// Clones share only the read-only original-file lease, never a document owner.
#[derive(Clone)]
pub struct CaptureSource {
    pub(in crate::desktop) session_meta: Option<serde_json::Value>,
    pub(in crate::desktop) original_lease: Option<std::sync::Arc<std::fs::File>>,
    pub original: Option<std::path::PathBuf>,
    pub reason: Option<crate::capture_dynamic::Reason>,
    pub downgraded: bool,
    pub display: Option<crate::capture_display::DisplayInfo>,
    pub diagnostic: Option<String>,
}
impl CaptureSource {
    pub(in crate::desktop) fn from_session(raw: &str) -> windows::core::Result<Self> {
        let metadata = crate::session_capture::decode(raw).map_err(|_| {
            windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid session capture metadata",
            )
        })?;
        let downgraded = metadata.downgraded_from_hdr.unwrap_or(false);
        let session_meta = serde_json::from_str(raw).map_err(|_| {
            windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid session capture metadata JSON",
            )
        })?;
        Ok(Self {
            session_meta: Some(session_meta),
            original_lease: None,
            original: None,
            reason: None,
            downgraded,
            display: None,
            diagnostic: None,
        })
    }
    pub(in crate::desktop) fn metadata(&self) -> serde_json::Value {
        self.session_meta.clone().unwrap_or_else(|| {
            let hdr = self.original.is_some();
            serde_json::json!({"dynamicRange": if hdr { "hdr" } else { "sdr" },
                "bitDepth": if hdr { 16 } else { 8 }, "colorSpace": if hdr { "bt2020-pq" } else { "srgb" },
                "downgradedFromHdr": self.downgraded })
        })
    }
    pub(in crate::desktop) fn original_label(&self) -> &'static str {
        if self.original_lease.is_some() {
            "HDR PNG"
        } else {
            match self
                .session_meta
                .as_ref()
                .and_then(|value| value.get("dynamicRange"))
                .and_then(serde_json::Value::as_str)
            {
                Some("hdr") => "HDR（原图文件未恢复）",
                Some("sdr") => "SDR",
                _ if self.session_meta.is_some() => "未记录",
                _ if self.original.is_some() => "HDR PNG",
                _ => "SDR",
            }
        }
    }
}
impl Canvas {
    pub(in crate::desktop) fn attach_capture(&mut self, source: Option<CaptureSource>) {
        self.capture_source = source.map(|source| (self.editor.document_stamp(), source));
    }
    /// 编辑后仍可追溯原始捕获，但不能把它当作已应用标注/裁剪的导出结果。
    pub fn capture_source(&self) -> Option<&CaptureSource> {
        self.capture_source
            .as_ref()
            .and_then(|(stamp, source)| stamp.same_document(&self.editor).then_some(source))
    }
}
