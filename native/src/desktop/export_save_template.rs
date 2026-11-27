//! A keyboard-accessible Win32 export sheet; system font and dialog units follow DPI.
use crate::ui_text::text as tr;
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn template() -> Vec<u32> {
    fn text(v: &mut Vec<u16>, s: &str) {
        v.extend(s.encode_utf16());
        v.push(0);
    }
    fn dword(v: &mut Vec<u16>, n: u32) {
        v.extend([n as u16, (n >> 16) as u16]);
    }
    let mut v = Vec::new();
    dword(
        &mut v,
        WS_POPUP.0 | WS_CAPTION.0 | WS_SYSMENU.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut v, 0);
    v.extend([8, 0, 0, 300, 118, 0, 0]);
    text(&mut v, tr("export-title"));
    v.push(9);
    text(&mut v, "Segoe UI");
    let radio = BS_AUTORADIOBUTTON as u32;
    for (id, class, style, x, y, w, h, s) in [
        (
            101,
            0x80,
            WS_GROUP.0 | WS_TABSTOP.0 | radio,
            10,
            10,
            42,
            14,
            "PNG",
        ),
        (102, 0x80, radio, 55, 10, 44, 14, "JPEG"),
        (
            103,
            0x80,
            radio,
            101,
            10,
            94,
            14,
            tr("export-webp-lossless"),
        ),
        (104, 0x80, radio, 199, 10, 97, 14, tr("export-webp-lossy")),
        (200, 0x82, WS_GROUP.0, 10, 28, 278, 14, ""),
        (201, 0x82, 0, 10, 64, 278, 24, ""),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_DEFPUSHBUTTON as u32,
            155,
            92,
            65,
            18,
            tr("common-ok"),
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0,
            226,
            92,
            64,
            18,
            tr("common-cancel"),
        ),
    ] {
        if v.len() % 2 != 0 {
            v.push(0);
        }
        dword(&mut v, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut v, 0);
        v.extend([x, y, w, h, id, 0xffff, class]);
        text(&mut v, s);
        v.push(0);
    }
    if v.len() % 2 != 0 {
        v.push(0);
    }
    v.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
