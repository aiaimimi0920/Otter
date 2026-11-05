//! 回收偏好原生对话框布局；只描述固定控件，设置修改由应用协调器拥有。
use windows::Win32::UI::WindowsAndMessaging::*;
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
    data.extend([7, 0, 0, 320, 146, 0, 0]);
    string(&mut data, "回收站保留设置");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    let edit = WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32;
    for (id, class, style, x, y, width, height, text) in [
        (201, 0x82, 0, 10, 12, 100, 14, "最多保留条数"),
        (101, 0x81, edit, 114, 10, 196, 16, ""),
        (202, 0x82, 0, 10, 36, 100, 14, "保留天数"),
        (102, 0x81, edit, 114, 34, 196, 16, ""),
        (103, 0x82, 0, 10, 60, 300, 46, "0 禁用相应限制；默认保留 15 条、不按天数过期。确认后裁剪当前回收站；保存会话后才写回。设置请用“保存设置”或自动保存持久化。"),
        (IDOK.0 as u16, 0x80, WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32, 190, 120, 55, 18, "确定"),
        (IDCANCEL.0 as u16, 0x80, WS_TABSTOP.0, 255, 120, 55, 18, "取消"),
    ] {
        if data.len() % 2 != 0 { data.push(0); }
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
        .map(|pair| u32::from(pair[0]) | (u32::from(pair[1]) << 16))
        .collect()
}
