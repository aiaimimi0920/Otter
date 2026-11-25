//! Scrolling-capture control windows use system dialog fonts and keyboard navigation.
use crate::ui_text::text as tr;
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
    data.extend([14, 0, 0, 380, 340, 0, 0]);
    string(&mut data, tr("long-capture-title"));
    data.push(9);
    string(&mut data, "Microsoft YaHei UI");
    for (id, class, style, x, y, width, height, text) in [
        (
            103,
            0x82,
            0,
            10,
            10,
            360,
            32,
            tr("long-capture-first-frame"),
        ),
        (
            104,
            0x82,
            windows::Win32::System::SystemServices::SS_OWNERDRAW.0,
            10,
            170,
            360,
            160,
            "",
        ),
        (
            102,
            0x80,
            WS_TABSTOP.0,
            10,
            50,
            55,
            20,
            tr("long-capture-preview"),
        ),
        (
            107,
            0x80,
            WS_TABSTOP.0,
            10,
            80,
            110,
            20,
            tr("long-capture-down"),
        ),
        (
            108,
            0x80,
            WS_TABSTOP.0,
            10,
            110,
            85,
            20,
            tr("long-capture-frame-up"),
        ),
        (
            109,
            0x80,
            WS_TABSTOP.0,
            102,
            110,
            85,
            20,
            tr("long-capture-frame-left"),
        ),
        (
            110,
            0x80,
            WS_TABSTOP.0,
            194,
            110,
            85,
            20,
            tr("long-capture-frame-right"),
        ),
        (
            111,
            0x80,
            WS_TABSTOP.0,
            286,
            110,
            84,
            20,
            tr("long-capture-frame-down"),
        ),
        (
            105,
            0x80,
            WS_TABSTOP.0,
            125,
            80,
            120,
            20,
            tr("long-capture-scroll"),
        ),
        (
            106,
            0x80,
            WS_TABSTOP.0,
            250,
            80,
            120,
            20,
            tr("long-capture-auto"),
        ),
        (
            112,
            0x80,
            WS_TABSTOP.0,
            10,
            140,
            360,
            20,
            tr("long-capture-record-stop"),
        ),
        (
            101,
            0x80,
            WS_TABSTOP.0,
            70,
            50,
            85,
            20,
            tr("long-capture-current-frame"),
        ),
        (
            1,
            0x80,
            WS_TABSTOP.0,
            160,
            50,
            125,
            20,
            tr("long-capture-finish-pin"),
        ),
        (2, 0x80, WS_TABSTOP.0, 290, 50, 80, 20, tr("common-cancel")),
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
