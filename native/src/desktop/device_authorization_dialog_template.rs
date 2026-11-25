//! Single service-address input layout; no browser, external fonts, or persistent render thread.
use crate::ui_text::text as tr;
use windows::Win32::UI::WindowsAndMessaging::*;
pub(super) fn template(art: bool) -> Vec<u32> {
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
        WS_POPUP.0 | WS_CLIPCHILDREN.0 | DS_MODALFRAME as u32 | DS_SETFONT as u32,
    );
    dword(&mut data, 0);
    data.extend([8, 0, 0, 340, 162, 0, 0]);
    string(
        &mut data,
        if art {
            tr("device-dialog-art-preview")
        } else {
            tr("device-dialog-connect-loom")
        },
    );
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (
            200,
            0x82,
            0,
            10,
            6,
            320,
            14,
            if art {
                tr("device-dialog-open-art")
            } else {
                tr("device-dialog-connect-loom")
            },
        ),
        (202, 0x82, 0, 10, 52, 112, 14, tr("device-dialog-code")),
        (
            102,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_PASSWORD as u32 | ES_AUTOHSCROLL as u32,
            126,
            50,
            204,
            18,
            "",
        ),
        (
            201,
            0x82,
            0,
            10,
            10,
            320,
            14,
            if art {
                "Art ID"
            } else {
                tr("device-dialog-endpoint")
            },
        ),
        (
            101,
            0x81,
            WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32,
            10,
            28,
            320,
            18,
            "",
        ),
        (103, 0x82, 0, 10, 78, 320, 28, ""),
        (
            IDOK.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            210,
            120,
            55,
            18,
            if art {
                tr("common-ok")
            } else {
                tr("device-dialog-connect")
            },
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            275,
            120,
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
        data.extend([
            x,
            if id == 200 { y } else { y + 14 },
            width,
            height,
            id,
            0xffff,
            class,
        ]);
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
