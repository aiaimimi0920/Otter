//! 仅无 raster/logical frame 的发送回退读取 HWND；复用原 Document 围栏，不新增窗口或持久化 owner。
use super::*;
use crate::{
    desktop::export::projection::Geometry, pin_geometry::PinRect, session_placement::Space,
};

pub(super) struct Captured {
    pub value: Geometry,
    document: restore_batch::guard::Document,
    space: Option<Space>,
}
impl Captured {
    pub(super) unsafe fn capture(
        doc: &DocumentWindow,
        kind: crate::projection_delivery::sender::Kind,
        space: Option<Space>,
    ) -> Result<Self> {
        let document = restore_batch::guard::Document::capture(doc)?;
        let canvas = doc.state.canvas.borrow();
        let value = if let Some(mini) = canvas
            .minified
            .as_ref()
            .filter(|_| matches!(kind, crate::projection_delivery::sender::Kind::Sticker))
        {
            if !mini.matches(&canvas.editor) {
                return Err(stale());
            }
            // annotations 是当前 expanded/savedRect 外框；image 可能是 contain/crop 源框。
            Geometry::Expanded(mini.geometry().0.annotations.size)
        } else {
            let mut rect = RECT::default();
            GetWindowRect(doc.window.0, &mut rect)?;
            Geometry::Window {
                frame: PinRect {
                    x: rect.left,
                    y: rect.top,
                    width: u32::try_from(i64::from(rect.right) - i64::from(rect.left))
                        .map_err(|_| stale())?,
                    height: u32::try_from(i64::from(rect.bottom) - i64::from(rect.top))
                        .map_err(|_| stale())?,
                },
                space: space.ok_or_else(stale)?,
            }
        };
        Ok(Self {
            value,
            document,
            space,
        })
    }
    pub(super) unsafe fn matches(&self, doc: &DocumentWindow, space: Option<Space>) -> bool {
        let same_space = match (self.space, space) {
            (Some(a), Some(b)) => a.origin == b.origin && a.scale == b.scale,
            (None, None) => true,
            _ => false,
        };
        same_space && self.document.matches(doc)
    }
}
