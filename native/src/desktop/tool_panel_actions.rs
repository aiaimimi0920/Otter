//! 工具面板的属性/文档操作目录；命令 ID 与菜单共享，标签只读取真实样式。
use super::{context_menu as menu, State};
use windows::{
    core::{Result, HSTRING},
    Win32::{Foundation::HWND, UI::WindowsAndMessaging::*},
};
pub(super) const ACTIONS: &[(usize, &str)] = &[
    (menu::COLOR, "颜色"),
    (menu::FILL_COLOR, "填充色"),
    (menu::WIDTH_DOWN, "粗细 −"),
    (menu::WIDTH_UP, "粗细 +"),
    (0x4F, "透明度"),
    (0x46, "填充"),
    (menu::APPLY_STYLE, "应用样式"),
    (menu::OPEN, "打开图片"),
    (super::copy_job::COMMAND, "复制图片"),
    (menu::EXPORT, "导出 PNG"),
    (super::numeric_style::WIDTH, "精确粗细…"),
    (super::numeric_style::OPACITY, "精确透明度…"),
    (super::capture_input::DEFAULT, "区域截图"),
    (super::blur_default::COMMAND, "新建模糊…"),
    (super::blur_numeric::COMMAND, "选中模糊…"),
    (super::numeric_style::MOSAIC, "色块大小…"),
];
// 可用性缓存有固定容量，新增目录项超限必须在编译期失败，而不是运行时截断高位。
const _: () = assert!(ACTIONS.len() + super::tool_commands::TOOLS.len() <= u64::BITS as usize);
pub(super) fn label(command: usize, state: &State) -> Option<String> {
    let name = ACTIONS.iter().find(|entry| entry.0 == command)?.1;
    let style = state.canvas.borrow().editor.style();
    Some(match command {
        menu::WIDTH_DOWN | menu::WIDTH_UP => format!("{name} {:.2}", style.width()),
        0x4F => format!("透明度 {:.0}%", style.opacity() * 100.0),
        0x46 => format!("填充 {}", if style.fill().is_some() { "开" } else { "关" }),
        _ => name.to_owned(),
    })
}
// Owner-draw 不会自动更新 HWND 文本。保持原生名称与绘制标签一致，且不重复发送 WM_SETTEXT。
pub(super) unsafe fn caption_matches(button: HWND, expected: &str) -> bool {
    let mut buffer = [0u16; 128];
    let length = GetWindowTextW(button, &mut buffer) as usize;
    buffer[..length].iter().copied().eq(expected.encode_utf16())
}
pub(super) unsafe fn sync_captions(panel: HWND, state: &State) -> Result<()> {
    for &(command, _) in ACTIONS {
        let button = GetDlgItem(Some(panel), command as i32)?;
        if let Some(text) = label(command, state) {
            if !caption_matches(button, &text) {
                SetWindowTextW(button, &HSTRING::from(text))?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_capture_button_creates_pin_instead_of_replacing_document() {
        assert_eq!(
            super::super::capture_input::DEFAULT,
            super::super::capture_input::NEW_PIN
        );
        assert!(ACTIONS
            .iter()
            .any(|entry| entry.0 == super::super::capture_input::DEFAULT && entry.1 == "区域截图"));
        assert!(!ACTIONS
            .iter()
            .any(|entry| entry.0 == super::super::capture_input::COMMAND));
    }
    #[test]
    fn commands_are_unique_and_do_not_shadow_tools() {
        assert!(ACTIONS.len() + super::super::tool_commands::TOOLS.len() <= u64::BITS as usize);
        for (index, (command, label)) in ACTIONS.iter().enumerate() {
            assert!(!label.is_empty());
            assert!(!ACTIONS[..index].iter().any(|entry| entry.0 == *command));
            assert!(!super::super::tool_commands::TOOLS
                .iter()
                .any(|entry| entry.id == *command));
        }
    }
}
