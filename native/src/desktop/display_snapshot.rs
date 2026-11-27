//! Current-display snapshots/freshness evidence for copy/export; snapshots share pixels while evidence retains no images.
use super::{canvas::Canvas, display_frame::Frame, image::Image};
use crate::editor::{DocumentStamp, Editor};
use std::sync::Arc;

/// Publishing only the last committed editor while a redaction is being
/// previewed could disclose exactly the pixels the user intended to remove.
pub(super) fn settled(c: &Canvas) -> bool {
    !c.display_waiting && locally_settled(c)
}

// Graph inputs refer to a local base/editor, not the producer's own resolved
// display. Including derived graph state here would make readiness order-dependent.
pub(super) fn locally_settled(c: &Canvas) -> bool {
    !c.blocks_wheel()
        && c.erase_pending.is_none()
        && !c.erase_display.as_ref().is_some_and(|r| {
            r.revision == c.editor.revision()
                && c.editor.layers().is_some_and(|l| l.same_storage(&r.source))
        })
        && c.scale_pending.is_none()
        && c.crop_pending.is_none()
        && c.group_translation.is_none()
        && c.group_preview.is_empty()
        && c.selection_effect_preview.is_none()
        && c.inline_text_hidden.is_none()
}

pub(super) fn ready(s: &super::State) -> bool {
    s.inline_text.borrow().is_none() && settled(&s.canvas.borrow())
}

#[derive(Clone)]
pub(super) struct Stamp {
    document: DocumentStamp,
    display: Option<Arc<()>>,
    opacity: f32,
    style: super::export::style::Snapshot,
}
impl Stamp {
    pub fn same(&self, other: &Self) -> bool {
        self.document.same_version(&other.document)
            && self.opacity == other.opacity
            && self.style == other.style
            && match (&self.display, &other.display) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
    pub fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Self {
        Self {
            document: canvas.editor.document_stamp(),
            display: frame.map(|frame| frame.identity.clone()),
            opacity: canvas.normal_opacity,
            style: super::export::style::Snapshot::capture_with_frame(canvas, frame),
        }
    }
    pub fn matches(&self, canvas: &Canvas, frame: Option<&Frame>) -> bool {
        self.document.matches(&canvas.editor)
            && self.opacity == canvas.normal_opacity
            && self.style == super::export::style::Snapshot::capture_with_frame(canvas, frame)
            && settled(canvas)
            && match (&self.display, frame) {
                (None, None) => true,
                (Some(identity), Some(frame)) => {
                    Arc::ptr_eq(identity, &frame.identity) && frame.current_producer()
                }
                _ => false,
            }
    }
}
pub(super) fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Option<(Image, Editor)> {
    if !settled(canvas) {
        return None;
    }
    if let Some(frame) = frame {
        if !frame.current_producer() {
            return None;
        }
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
