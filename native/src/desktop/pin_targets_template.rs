//! Declarative control template for the target-selection popup; Window manages layout and business state.
use super::*;
unsafe fn refresh(hwnd: HWND) -> Result<()> {
    SetWindowTextW(
        hwnd,
        &HSTRING::from(crate::ui_text::text("pin-projection-targets")),
    )?;
    for (control, id) in [
        (scroll::ID, "pin-target-scroll"),
        (4, "pin-target-all"),
        (3, "pin-target-refresh"),
        (1, "pin-target-confirm"),
    ] {
        SetDlgItemTextW(hwnd, control, &HSTRING::from(crate::ui_text::text(id)))?;
    }
    Ok(())
}
pub(super) unsafe fn refresh_if_changed(model: &Model, hwnd: HWND) {
    if model.language.get() == crate::ui_text::current_language() {
        return;
    }
    match refresh(hwnd) {
        Ok(()) => model.language.set(crate::ui_text::current_language()),
        Err(error) => eprintln!("native-target-language: {error}"),
    }
}
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
    text(&mut out, crate::ui_text::text("pin-projection-targets"));
    out.push(9);
    text(&mut out, "Microsoft YaHei UI");
    for (id, class, style, label) in [
        (
            scroll::ID as u16,
            0x82,
            windows::Win32::System::SystemServices::SS_OWNERDRAW.0
                | windows::Win32::System::SystemServices::SS_NOTIFY.0,
            crate::ui_text::text("pin-target-scroll"),
        ),
        (
            4,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            crate::ui_text::text("pin-target-all"),
        ),
        (
            3,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            crate::ui_text::text("pin-target-refresh"),
        ),
        (
            1,
            0x80,
            WS_TABSTOP.0 | BS_OWNERDRAW as u32,
            crate::ui_text::text("pin-target-confirm"),
        ),
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
