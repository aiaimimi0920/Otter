//! Current-display snapshots/freshness evidence for copy/export; snapshots share pixels while evidence retains no images.
use super::{canvas::Canvas, display_frame::Frame, image::Image};
use crate::editor::{DocumentStamp, Editor};
use std::sync::Arc;

pub(super) struct Stamp {
    document: DocumentStamp,
    display: Option<Arc<()>>,
    opacity: f32,
    style: super::export::style::Snapshot,
}
impl Stamp {
    pub fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Self {
        Self {
            document: canvas.editor.document_stamp(),
            display: frame.map(|frame| frame.identity.clone()),
            opacity: canvas.normal_opacity,
            style: super::export::style::Snapshot::capture(canvas),
        }
    }
    pub fn matches(&self, canvas: &Canvas, frame: Option<&Frame>) -> bool {
        self.document.matches(&canvas.editor)
            && self.opacity == canvas.normal_opacity
            && self.style == super::export::style::Snapshot::capture(canvas)
            && canvas.editor.draft().is_none()
            && match (&self.display, frame) {
                (None, None) => true,
                (Some(identity), Some(frame)) => Arc::ptr_eq(identity, &frame.identity),
                _ => false,
            }
    }
}
pub(super) fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Option<(Image, Editor)> {
    if canvas.editor.draft().is_some() {
        return None;
    }
    if let Some(frame) = frame {
        Some((
            frame.image.clone(),
            frame
                .editor
                .as_deref()
                .map(Editor::export_snapshot)
                .unwrap_or_default(),
        ))
    } else {
        Some((
            canvas.image.as_ref()?.clone(),
            canvas.editor.export_snapshot(),
        ))
    }
}

/// Share dimension admission with PNG composition; no editor-history or image-buffer cloning.
pub(super) fn dimensions(canvas: &Canvas, frame: Option<&Frame>) -> Option<(u32, u32)> {
    if let Some(frame) = frame {
        super::export::dimensions(&frame.image, frame.editor.as_deref()).ok()
    } else {
        super::export::dimensions(canvas.image.as_ref()?, Some(&canvas.editor)).ok()
    }
}
