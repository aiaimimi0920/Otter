//! Paint only empty-field hints without writing EDIT text; support classic EDIT and multiline fields without requiring a v6 manifest.
use std::{cell::Cell, rc::Rc};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{
            Controls::{EM_GETRECT, EM_REPLACESEL},
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};
const ID: usize = 0x4f545450;
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Edit.Placeholder.Owner");
struct Hint {
    text: Vec<u16>,
    multiline: bool,
    composing: Cell<bool>,
}
pub fn install(hwnd: HWND, text: &str, multiline: bool) -> Result<()> {
    if text.len() > super::MAX_BYTES
        || text.encode_utf16().count() > super::MAX_UNITS
        || text.contains('\0')
    {
        return Err(Error::new(E_INVALIDARG, "Invalid edit placeholder"));
    }
    if text.is_empty() {
        return Ok(());
    }
    let text = if multiline {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.replace(['\r', '\n'], "")
    };
    let hint = Rc::new(Hint {
        text: text.encode_utf16().collect(),
        multiline,
        composing: Cell::new(false),
    });
    unsafe {
        if !GetPropW(hwnd, OWNER).is_invalid() {
            return Err(Error::new(E_INVALIDARG, "Placeholder already installed"));
        }
        let pointer = Rc::into_raw(hint);
        if let Err(error) = SetPropW(hwnd, OWNER, Some(HANDLE(pointer.cast_mut().cast()))) {
            drop(Rc::from_raw(pointer));
            return Err(error);
        }
        if !SetWindowSubclass(hwnd, Some(procedure), ID, pointer as usize).as_bool() {
            let _ = RemovePropW(hwnd, OWNER);
            drop(Rc::from_raw(pointer));
            return Err(Error::from_win32());
        }
    }
    Ok(())
}
unsafe fn paint(hwnd: HWND, dc: HDC, hint: &Hint) {
    if dc.is_invalid() || hint.composing.get() || GetWindowTextLengthW(hwnd) != 0 {
        return;
    }
    let saved = SaveDC(dc);
    if saved == 0 {
        return;
    }
    let mut rect = RECT::default();
    SendMessageW(
        hwnd,
        EM_GETRECT,
        None,
        Some(LPARAM((&mut rect as *mut RECT) as isize)),
    );
    IntersectClipRect(dc, rect.left, rect.top, rect.right, rect.bottom);
    let font = SendMessageW(hwnd, WM_GETFONT, None, None);
    if font.0 != 0 {
        SelectObject(dc, HGDIOBJ(font.0 as *mut _));
    }
    SetBkMode(dc, TRANSPARENT);
    // Neuro textMuted #929a9f. This hint is not a disabled value; disabled state must not reduce it to unreadable contrast.
    SetTextColor(dc, COLORREF(0x9f9a92));
    // DrawTextW's mutable slice API requires an independent buffer; paint only empty fields, with size bounded by field budgets.
    let mut text = hint.text.clone();
    DrawTextW(
        dc,
        &mut text,
        &mut rect,
        DT_NOPREFIX
            | match GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & ((ES_CENTER | ES_RIGHT) as u32) {
                value if value == ES_CENTER as u32 => DT_CENTER,
                value if value == ES_RIGHT as u32 => DT_RIGHT,
                _ => DT_LEFT,
            }
            | if hint.multiline {
                DT_WORDBREAK
            } else {
                DT_SINGLELINE
            },
    );
    let _ = RestoreDC(dc, saved);
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let pointer = data as *const Hint;
    // Temporarily retain a strong reference per callback; default handling may reenter/destroy the window without freeing current hint data.
    Rc::increment_strong_count(pointer);
    let hint = Rc::from_raw(pointer);
    if message == WM_NCDESTROY {
        let _ = RemovePropW(hwnd, OWNER);
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
        drop(Rc::from_raw(pointer)); // Release the window-held reference; the current callback still holds hint.
        return DefSubclassProc(hwnd, message, wp, lp);
    }
    if message == WM_IME_STARTCOMPOSITION {
        hint.composing.set(true);
    }
    if message == WM_IME_ENDCOMPOSITION || (message == WM_ENABLE && wp.0 == 0) {
        hint.composing.set(false);
    }
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if GetPropW(hwnd, OWNER).0 as usize != data {
        return result;
    }
    if matches!(
        message,
        WM_IME_STARTCOMPOSITION
            | WM_IME_ENDCOMPOSITION
            | WM_IME_COMPOSITION
            | WM_SETTEXT
            | WM_CHAR
            | WM_UNDO
            | EM_REPLACESEL
            | WM_CUT
            | WM_PASTE
            | WM_CLEAR
            | WM_SETFOCUS
            | WM_KILLFOCUS
            | WM_ENABLE
    ) {
        let _ = InvalidateRect(Some(hwnd), None, true);
    }
    if message == WM_PAINT {
        let dc = GetDC(Some(hwnd));
        paint(hwnd, dc, &hint);
        if !dc.is_invalid() {
            ReleaseDC(Some(hwnd), dc);
        }
    } else if message == WM_PRINTCLIENT && wp.0 != 0 {
        paint(hwnd, HDC(wp.0 as *mut _), &hint);
    }
    result
}
