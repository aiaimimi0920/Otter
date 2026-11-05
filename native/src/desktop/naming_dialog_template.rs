//! Win32 对话框布局；以 dialog units 和系统字体布局，控件按字段顺序参与键盘导航。
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
    data.extend([11, 0, 0, 360, 182, 0, 0]);
    string(&mut data, "文件命名设置");
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    let edit = WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32;
    for (id, class, style, x, y, width, height, text) in [
        (201, 0x82, 0, 10, 12, 90, 14, "PNG 保存模板"),
        (101, 0x81, edit, 104, 10, 246, 16, ""),
        (202, 0x82, 0, 10, 36, 90, 14, "拖出文件模板"),
        (102, 0x81, edit, 104, 34, 246, 16, ""),
        (203, 0x82, 0, 10, 60, 90, 14, "剪贴板文件模板"),
        (103, 0x81, edit, 104, 58, 246, 16, ""),
        (204, 0x82, 0, 10, 84, 90, 14, "标题最大长度"),
        (104, 0x81, edit, 104, 82, 246, 16, ""),
        (105, 0x82, 0, 10, 106, 340, 42, "占位符：{app} {kind} {label} {title} {process} {unitId} {shortId} {width} {height} {date} {time} {timestamp}"),
        (IDOK.0 as u16, 0x80, WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32, 230, 156, 55, 18, "确定"),
        (IDCANCEL.0 as u16, 0x80, WS_TABSTOP.0, 295, 156, 55, 18, "取消"),
    ] {
        if data.len() % 2 != 0 { data.push(0); }
        dword(&mut data, WS_CHILD.0 | WS_VISIBLE.0 | style); dword(&mut data, 0);
        data.extend([x, y, width, height, id, 0xffff, class]);
        string(&mut data, text); data.push(0);
    }
    if data.len() % 2 != 0 {
        data.push(0);
    }
    data.chunks_exact(2)
        .map(|pair| u32::from(pair[0]) | (u32::from(pair[1]) << 16))
        .collect()
}
