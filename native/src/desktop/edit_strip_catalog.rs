//! 主干主工具条的分组与图标；业务命令留在 Otter，主题库只消费通用图标和按钮。
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
        label,
        icon: icon(command),
        dropdown,
    })
}
pub(super) fn items() -> Vec<Item> {
    let mut items = vec![
        button(SETTINGS, "钉图设置", 0, false),
        button(MODE, "选择", 0x53, false),
        button(0x1007, "撤销", 0x1007, false),
        button(0x1008, "重做", 0x1008, false),
        button(RASTER, "栅格化", 0x1022, true),
        button(OCR, "识别结果", OCR as usize, false),
        button(TRANSLATE, "翻译结果", TRANSLATE as usize, false),
        Item::Divider,
        button(SHAPE, "矩形", 0x52, true),
        button(LINE, "直线", 0x4c, true),
        button(BRUSH, "铅笔", 0x42, true),
        button(LABEL, "文本", 0x58, true),
        button(EFFECT, "马赛克", 0x4d, true),
        button(0x57, "橡皮擦", 0x57, false),
        button(0x102e, "裁剪", 0x102e, false),
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
        EFFECT => &[0x4d, 0x47],
        RASTER => &[0x1022, 0x1023],
        _ => &[],
    }
}
