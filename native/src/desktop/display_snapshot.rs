//! 复制/导出的当前显示快照及新鲜度凭证；快照共享像素，凭证不保留图片。
use super::{canvas::Canvas, display_frame::Frame, image::Image};
use crate::editor::{DocumentStamp, Editor};
use std::sync::Arc;

pub(super) struct Stamp {
    document: DocumentStamp,
    display: Option<Arc<()>>,
    opacity: f32,
}
impl Stamp {
    pub fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Self {
        Self {
            document: canvas.editor.document_stamp(),
            display: frame.map(|frame| frame.identity.clone()),
            opacity: canvas.normal_opacity,
        }
    }
    pub fn matches(&self, canvas: &Canvas, frame: Option<&Frame>) -> bool {
        self.document.matches(&canvas.editor)
            && self.opacity == canvas.normal_opacity
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

/// 与 PNG 合成共用尺寸准入；不克隆编辑器历史或图像缓冲。
pub(super) fn dimensions(canvas: &Canvas, frame: Option<&Frame>) -> Option<(u32, u32)> {
    if let Some(frame) = frame {
        super::export::dimensions(&frame.image, frame.editor.as_deref()).ok()
    } else {
        super::export::dimensions(canvas.image.as_ref()?, Some(&canvas.editor)).ok()
    }
}
