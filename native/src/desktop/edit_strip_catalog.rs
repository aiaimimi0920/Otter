//! Baseline primary-toolbar groups/icons; Otter retains business commands, while theme libraries consume only generic icons/buttons.
use neuro_mini_ui::{Button, Item};
pub(super) const SETTINGS: u16 = 0x6100;
pub(super) const MODE: u16 = 0x6101;
pub(super) const SHAPE: u16 = 0x6102;
pub(super) const LINE: u16 = 0x6103;
pub(super) const BRUSH: u16 = 0x6104;
pub(super) const LABEL: u16 = 0x6105;
pub(super) const EFFECT: u16 = 0x6106;
pub(super) const RASTER: u16 = 0x6107;
pub(super) const OCR: u16 = 0x6108;
pub(super) const TRANSLATE: u16 = 0x6109;
pub(super) use super::icons::icon;
fn button(id: u16, label: &'static str, command: usize, dropdown: bool) -> Item {
    Item::Button(Button {
        id,
        label: crate::ui_text::text(label),
        icon: icon(command),
        dropdown,
    })
}
pub(super) fn items() -> Vec<Item> {
    let mut items = vec![
        button(SETTINGS, "edit-pin-settings", 0, false),
        button(MODE, "edit-select", 0x53, false),
        button(0x1007, "common-undo", 0x1007, false),
        button(0x1008, "common-redo", 0x1008, false),
        button(RASTER, "edit-rasterize", 0x1022, true),
        button(OCR, "edit-ocr-result", OCR as usize, false),
        button(
            TRANSLATE,
            "edit-translation-result",
            TRANSLATE as usize,
            false,
        ),
        Item::Divider,
        button(SHAPE, "edit-rectangle", 0x52, true),
        button(LINE, "edit-line", 0x4c, true),
        button(BRUSH, "edit-pencil", 0x42, true),
        button(LABEL, "edit-text", 0x58, true),
        button(EFFECT, "edit-mosaic", 0x4d, true),
        button(0x57, "edit-eraser", 0x57, false),
        button(0x102e, "edit-crop", 0x102e, false),
        Item::Divider,
    ];
    items.extend((0x6300..0x6308).map(Item::Slot));
    items
}
pub(super) fn choices(id: u16) -> &'static [usize] {
    match id {
        SHAPE => &[0x52, 0x45, 0x54, 0x50],
        LINE => &[0x4c, 0x4b],
        BRUSH => &[0x42, 0x4a],
        LABEL => &[0x58, 0x4e],
        EFFECT => &[0x4d, 0x47, 0x21f1, 0x21f4],
        RASTER => &[0x1022, 0x1023],
        _ => &[],
    }
}
