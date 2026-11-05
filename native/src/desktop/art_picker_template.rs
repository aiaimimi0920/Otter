//! 标准系统对话框布局；只有 EDIT、LISTBOX 和操作，不复制能力业务或持久化数据。
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
    data.extend([6, 0, 0, 340, 255, 0, 0]);
    string(&mut data, "添加 Art");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, w, h, text) in [
        (
            SEARCH,
            0x81,
            WS_TABSTOP.0 | ES_AUTOHSCROLL as u32 | WS_BORDER.0,
            8,
            8,
            324,
            20,
            "",
        ),
        (
            LIST,
            0x83,
            WS_TABSTOP.0 | WS_VSCROLL.0 | WS_BORDER.0 | LBS_NOTIFY as u32,
            8,
            34,
            324,
            158,
            "",
        ),
        (STATUS, 0x82, 0, 8, 198, 324, 18, ""),
        (REFRESH, 0x80, WS_TABSTOP.0, 8, 226, 74, 20, "刷新列表"),
        (
            IDOK.0,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            204,
            226,
            60,
            20,
            "添加",
        ),
        (IDCANCEL.0, 0x80, WS_TABSTOP.0, 272, 226, 60, 20, "取消"),
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
