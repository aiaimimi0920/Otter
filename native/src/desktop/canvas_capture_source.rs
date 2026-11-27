//! Capture provenance binds only current document identity; paths record origin, not file read/write/delete authority or current edited output.
use super::Canvas;
// Clones share only the read-only original-file lease, never a document owner.
#[derive(Clone)]
pub struct CaptureSource {
    pub(in crate::desktop) naming: Option<crate::desktop::capture_window_meta::Meta>,
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
            naming: None,
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
                Some("hdr") => crate::ui_text::text("action-hdr-unrestored"),
                Some("sdr") => "SDR",
                _ if self.session_meta.is_some() => {
                    crate::ui_text::text("action-capture-unrecorded")
                }
                _ if self.original.is_some() => "HDR PNG",
                _ => "SDR",
            }
        }
    }
}
impl Canvas {
    // A naming-only Live snapshot must never synthesize persisted capture_meta.
    pub(in crate::desktop) fn bind_live_naming(
        &mut self,
        hint: Option<crate::desktop::capture_window_meta::Meta>,
    ) {
        self.live_naming = hint.map(|h| (self.editor.document_stamp(), h));
    }
    pub(in crate::desktop) fn source_naming(
        &self,
    ) -> Option<&crate::desktop::capture_window_meta::Meta> {
        self.live_naming
            .as_ref()
            .and_then(|(s, h)| s.same_document(&self.editor).then_some(h))
            .or_else(|| self.capture_source().and_then(|s| s.naming.as_ref()))
    }
    pub(in crate::desktop) fn attach_capture(&mut self, source: Option<CaptureSource>) {
        self.capture_source = source.map(|source| (self.editor.document_stamp(), source));
    }
    /// Edits retain traceability to the original capture, but it is not an export with annotations/cropping applied.
    pub fn capture_source(&self) -> Option<&CaptureSource> {
        self.capture_source
            .as_ref()
            .and_then(|(stamp, source)| stamp.same_document(&self.editor).then_some(source))
    }
}
