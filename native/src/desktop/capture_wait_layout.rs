//! 仅状态和取消按钮，不增加持续动画或解释型文案。
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn template() -> Vec<u32> {
    fn text(data: &mut Vec<u16>, value: &str) {
        data.extend(value.encode_utf16());
        data.push(0);
    }
    fn word(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    word(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    word(&mut data, WS_EX_TOPMOST.0);
    data.extend([2, 0, 0, 180, 65, 0, 0]);
    text(&mut data, "截图");
    data.push(9);
    text(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, label) in [
        (103, 0x82, 0, 10, 10, 160, 18, "正在截图…"),
        (2, 0x80, WS_TABSTOP.0, 115, 35, 55, 20, "取消"),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        word(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        word(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        text(&mut data, label);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|p| u32::from(p[0]) | u32::from(p[1]) << 16)
        .collect()
}
