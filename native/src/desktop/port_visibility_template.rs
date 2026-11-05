//! 标准有界列表/选择控件；沿Win32 Dialog键盘、Enter/Escape和owner模态生命周期。
use super::*;
pub(super) fn template() -> Vec<u32> {
    fn string(data: &mut Vec<u16>, text: &str) {
        data.extend(text.encode_utf16());
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
    dword(&mut data, 0);
    data.extend([6, 0, 0, 310, 238, 0, 0]);
    string(&mut data, "端口显示");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (
            PORTS,
            0x83,
            WS_TABSTOP.0
                | WS_BORDER.0
                | WS_VSCROLL.0
                | LBS_NOTIFY as u32
                | LBS_NOINTEGRALHEIGHT as u32,
            10,
            10,
            290,
            152,
            "",
        ),
        (DEFAULT, 0x82, 0, 10, 170, 130, 14, ""),
        (
            MODE,
            0x85,
            WS_TABSTOP.0 | WS_VSCROLL.0 | CBS_DROPDOWNLIST as u32,
            145,
            168,
            155,
            80,
            "",
        ),
        (STATUS, 0x82, 0, 10, 192, 290, 16, ""),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            180,
            212,
            55,
            18,
            "确定",
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            245,
            212,
            55,
            18,
            "取消",
        ),
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
        .map(|v| v[0] as u32 | ((v[1] as u32) << 16))
        .collect()
}
