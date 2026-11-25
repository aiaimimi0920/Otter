//! Drag-out snapshots encode PNG only, without clipboard-only DIB allocation; reuse display opacity and annotation composition.
use super::*;

pub(in crate::desktop) fn png_with_opacity(
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> Result<(crate::pixels::Pixels, crate::desktop::drag_preview::Preview)> {
    style::Snapshot::default().png(image, editor, opacity)
}
