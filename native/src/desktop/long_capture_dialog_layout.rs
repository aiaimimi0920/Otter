//! 长截图控制窗口，使用系统对话框字体及键盘导航。
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn template() -> Vec<u32> {
    fn string(data: &mut Vec<u16>, value: &str) {
        data.extend(value.encode_utf16());
        data.push(0);
    }
    fn dword(data: &mut Vec<u16>, value: u32) {
        data.extend([value as u16, (value >> 16) as u16]);
    }
    let mut data = Vec::new();
    dword(
        &mut data,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, WS_EX_TOPMOST.0);
    data.extend([14, 0, 0, 300, 340, 0, 0]);
    string(&mut data, "长截图");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (103, 0x82, 0, 10, 10, 280, 24, "已采集 1 帧"),
        (
            104,
            0x82,
            windows::Win32::System::SystemServices::SS_OWNERDRAW.0,
            10,
            170,
            280,
            160,
            "",
        ),
        (102, 0x80, WS_TABSTOP.0, 10, 50, 45, 20, "预览"),
        (107, 0x80, WS_TABSTOP.0, 10, 80, 75, 20, "方向：向下"),
        (108, 0x80, WS_TABSTOP.0, 10, 110, 65, 20, "取景框上移"),
        (109, 0x80, WS_TABSTOP.0, 82, 110, 65, 20, "取景框左移"),
        (110, 0x80, WS_TABSTOP.0, 154, 110, 65, 20, "取景框右移"),
        (111, 0x80, WS_TABSTOP.0, 226, 110, 64, 20, "取景框下移"),
        (105, 0x80, WS_TABSTOP.0, 90, 80, 95, 20, "滚动并采集"),
        (106, 0x80, WS_TABSTOP.0, 190, 80, 100, 20, "自动采集"),
        (112, 0x80, WS_TABSTOP.0, 10, 140, 280, 20, "停止录制"),
        (101, 0x80, WS_TABSTOP.0, 60, 50, 65, 20, "采集当前帧"),
        (1, 0x80, WS_TABSTOP.0, 130, 50, 75, 20, "完成为新钉图"),
        (2, 0x80, WS_TABSTOP.0, 215, 50, 75, 20, "取消"),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
