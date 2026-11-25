//! Declarative control template for the target-selection popup; Window manages layout and business state.
use super::*;
pub(super) fn template() -> Vec<u32> {
    fn dword(out: &mut Vec<u16>, v: u32) {
        out.extend([v as u16, (v >> 16) as u16]);
    }
    fn text(out: &mut Vec<u16>, v: &str) {
        out.extend(v.encode_utf16());
        out.push(0);
    }
    let mut out = Vec::new();
    dword(&mut out, WS_POPUP.0 | WS_CLIPCHILDREN.0 | DS_SETFONT as u32);
    dword(&mut out, WS_EX_TOOLWINDOW.0);
    out.extend([6, 0, 0, 176, 200, 0, 0]);
    text(&mut out, "投射目标");
    out.push(9);
    text(&mut out, "Microsoft YaHei UI");
    for (id, class, style, label) in [
        (
            scroll::ID as u16,
            0x82,
            windows::Win32::System::SystemServices::SS_OWNERDRAW.0
                | windows::Win32::System::SystemServices::SS_NOTIFY.0,
            "滚动目标",
        ),
        (4, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "全部"),
        (3, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "刷新"),
        (1, 0x80, WS_TABSTOP.0 | BS_OWNERDRAW as u32, "确认"),
        (
            11,
            0x82,
            windows::Win32::System::SystemServices::SS_ENDELLIPSIS.0,
            "",
        ),
        (
            LIST as u16,
            0x83,
            WS_TABSTOP.0
                | LBS_NOTIFY as u32
                | LBS_MULTIPLESEL as u32
                | LBS_OWNERDRAWFIXED as u32
                | LBS_HASSTRINGS as u32
                | LBS_NOINTEGRALHEIGHT as u32,
            "",
        ),
    ] {
        if out.len() % 2 != 0 {
            out.push(0);
        }
        dword(&mut out, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut out, 0);
        out.extend([0, 0, 1, 1, id, 0xffff, class]);
        text(&mut out, label);
        out.push(0);
    }
    if out.len() % 2 != 0 {
        out.push(0);
    }
    out.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
