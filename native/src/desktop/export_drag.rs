//! 拖出快照只编码 PNG，不分配剪贴板专用 DIB；复用显示透明度与标注合成。
use super::*;

pub(in crate::desktop) fn png_with_opacity(
    image: &Image,
    editor: &Editor,
    opacity: f32,
) -> Result<(crate::pixels::Pixels, crate::desktop::drag_preview::Preview)> {
    style::Snapshot::default().png(image, editor, opacity)
}
