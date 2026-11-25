//! Win32 dialog layout in dialog units/system fonts; controls participate in keyboard navigation in field order.
use crate::ui_text::text as tr;
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
    string(&mut data, tr("naming-title"));
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    let edit = WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32;
    for (id, class, style, x, y, width, height, text) in [
        (201, 0x82, 0, 10, 12, 90, 14, tr("naming-save-template")),
        (101, 0x81, edit, 104, 10, 246, 16, ""),
        (202, 0x82, 0, 10, 36, 90, 14, tr("naming-drag-template")),
        (102, 0x81, edit, 104, 34, 246, 16, ""),
        (
            203,
            0x82,
            0,
            10,
            60,
            90,
            14,
            tr("naming-clipboard-template"),
        ),
        (103, 0x81, edit, 104, 58, 246, 16, ""),
        (204, 0x82, 0, 10, 84, 90, 14, tr("naming-title-length")),
        (104, 0x81, edit, 104, 82, 246, 16, ""),
        (105, 0x82, 0, 10, 106, 340, 42, tr("naming-placeholders")),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            230,
            156,
            55,
            18,
            tr("common-ok"),
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            295,
            156,
            55,
            18,
            tr("common-cancel"),
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
        .map(|pair| u32::from(pair[0]) | (u32::from(pair[1]) << 16))
        .collect()
}
