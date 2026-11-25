//! Popup-menu target snapshots; reject changed documents/selections after nested message loops.
use crate::{
    desktop::canvas::Canvas,
    editor::{DocumentStamp, Tool},
    stroke_style::StrokeStyle,
};

#[derive(Clone)]
pub(in crate::desktop) struct MenuTarget {
    stamp: DocumentStamp,
    tool: Tool,
    selection: Vec<usize>,
    style: StrokeStyle,
    crop_mode: bool,
}
impl MenuTarget {
    pub(in crate::desktop) fn capture(canvas: &Canvas) -> Self {
        Self {
            stamp: canvas.editor.document_stamp(),
            tool: canvas.tool,
            selection: canvas.selected_indices(),
            style: canvas.editor.style(),
            crop_mode: canvas.crop_mode,
        }
    }
    pub(in crate::desktop) fn matches(&self, canvas: &Canvas) -> bool {
        self.stamp.matches(&canvas.editor)
            && self.tool == canvas.tool
            && self.selection == canvas.selected_indices()
            && self.style == canvas.editor.style()
            && self.crop_mode == canvas.crop_mode
    }
}
