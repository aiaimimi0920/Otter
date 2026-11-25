//! Compact titleless search menu; DialogBox retains modal, focus, and exit ownership, without duplicating capability business logic.
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
        WS_POPUP.0 | WS_CLIPCHILDREN.0 | DS_SETFONT as u32,
    );
    dword(&mut data, WS_EX_TOOLWINDOW.0);
    data.extend([6, 0, 0, 180, 210, 0, 0]);
    string(&mut data, "添加 Art");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, w, h, text) in [
        (
            SEARCH,
            0x81,
            WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            8,
            8,
            324,
            20,
            "",
        ),
        (
            LIST,
            0x83,
            WS_TABSTOP.0
                | WS_VSCROLL.0
                | LBS_NOTIFY as u32
                | LBS_OWNERDRAWFIXED as u32
                | LBS_HASSTRINGS as u32
                | LBS_NOINTEGRALHEIGHT as u32,
            8,
            34,
            324,
            158,
            "",
        ),
        (STATUS, 0x82, 0, 8, 198, 324, 18, ""),
        (
            REFRESH,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            8,
            226,
            74,
            20,
            "刷新列表",
        ),
        (
            CLEAR,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            204,
            8,
            24,
            20,
            "清空搜索",
        ),
        (
            IDCANCEL.0,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            272,
            8,
            28,
            20,
            "关闭 Art 菜单",
        ),
    ] {
        if data.len() % 2 != 0 {
            data.push(0);
        }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut data, 0);
        data.extend([x, y, w, h, id as u16, 0xffff, class]);
        string(&mut data, text);
        data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|p| p[0] as u32 | ((p[1] as u32) << 16))
        .collect()
}
