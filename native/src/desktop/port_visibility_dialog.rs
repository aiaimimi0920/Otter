//! 单次模态端口草稿；不借用Application，不在窗口回调修改Workspace或图。
use super::*;
use crate::desktop::document_window::parameter_panel::theme;
use std::cell::{Cell, RefCell};
use windows::Win32::{Graphics::Gdi::*, UI::Controls::*};
#[path = "port_visibility_model.rs"]
mod model;
#[path = "port_visibility_template.rs"]
mod template;
pub(super) use model::Model;
const PORTS: u16 = 101;
const MODE: u16 = 102;
const DEFAULT: u16 = 103;
const STATUS: u16 = 104;
// x64的DWL_USER仍为8；Dialog指针位于消息结果和DLGPROC之后，不可沿用32位偏移。
const DWLP_USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
struct Dialog {
    model: RefCell<Model>,
    accepted: Cell<bool>,
    protected: bool,
    error: RefCell<Option<Error>>,
}
pub(super) unsafe fn show(
    owner: HWND,
    model: Model,
    protected: bool,
) -> Result<Option<crate::workspace::ports::Visibility>> {
    let dialog = Dialog {
        model: RefCell::new(model),
        accepted: Cell::new(false),
        protected,
        error: RefCell::default(),
    };
    let data = template::template();
    let result = DialogBoxIndirectParamW(
        None,
        data.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&dialog as *const Dialog as isize),
    );
    if let Some(error) = dialog.error.into_inner() {
        return Err(error);
    }
    if result == -1 {
        return Err(Error::from_win32());
    }
    Ok(dialog
        .accepted
        .get()
        .then(|| dialog.model.into_inner().visibility))
}
unsafe fn child(hwnd: HWND, id: u16) -> HWND {
    GetDlgItem(Some(hwnd), id as i32).unwrap_or_default()
}
unsafe fn selected(hwnd: HWND, id: u16, message: u32) -> Option<usize> {
    usize::try_from(SendMessageW(child(hwnd, id), message, None, None).0).ok()
}
unsafe fn refresh(hwnd: HWND, dialog: &Dialog) {
    let Some(index) = selected(hwnd, PORTS, LB_GETCURSEL) else {
        return;
    };
    let (mode, default) = {
        let model = dialog.model.borrow();
        let Some(row) = model.rows.get(index) else {
            return;
        };
        (model.mode(index).unwrap(), row.default_visible)
    };
    SendMessageW(child(hwnd, MODE), CB_SETCURSEL, Some(WPARAM(mode)), None);
    let _ = SetWindowTextW(
        child(hwnd, DEFAULT),
        &HSTRING::from(if default {
            "默认：显示"
        } else {
            "默认：隐藏"
        }),
    );
}
unsafe fn init(hwnd: HWND, dialog: &Dialog) -> Result<()> {
    SetWindowDisplayAffinity(
        hwnd,
        if dialog.protected {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        },
    )?;
    crate::desktop::tool_panel::font::update(
        hwnd,
        windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd),
    )?;
    let labels: Vec<_> = dialog
        .model
        .borrow()
        .rows
        .iter()
        .map(|r| r.label.clone())
        .collect();
    for label in labels {
        if SendMessageW(
            child(hwnd, PORTS),
            LB_ADDSTRING,
            None,
            Some(LPARAM(HSTRING::from(label).as_ptr() as isize)),
        )
        .0 < 0
        {
            return Err(Error::from(E_OUTOFMEMORY));
        }
    }
    for label in ["使用默认值", "显示", "隐藏"] {
        if SendMessageW(
            child(hwnd, MODE),
            CB_ADDSTRING,
            None,
            Some(LPARAM(HSTRING::from(label).as_ptr() as isize)),
        )
        .0 < 0
        {
            return Err(Error::from(E_OUTOFMEMORY));
        }
    }
    SendMessageW(child(hwnd, PORTS), LB_SETCURSEL, Some(WPARAM(0)), None);
    SendMessageW(hwnd, DM_SETDEFID, Some(WPARAM(IDOK.0 as usize)), None);
    fonts(hwnd);
    refresh(hwnd, dialog);
    Ok(())
}
unsafe fn fonts(hwnd: HWND) {
    for id in [
        PORTS,
        MODE,
        DEFAULT,
        STATUS,
        IDOK.0 as u16,
        IDCANCEL.0 as u16,
    ] {
        SendMessageW(
            child(hwnd, id),
            WM_SETFONT,
            Some(WPARAM(
                crate::desktop::tool_panel::font::object(hwnd).0 as usize,
            )),
            Some(LPARAM(1)),
        );
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    std::panic::catch_unwind(|| dispatch(hwnd, msg, wp, lp)).unwrap_or(0)
}
unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, DWLP_USER, lp.0);
    }
    let pointer = GetWindowLongPtrW(hwnd, DWLP_USER) as *const Dialog;
    if pointer.is_null() {
        return 0;
    }
    let dialog = &*pointer;
    if msg == WM_NCDESTROY {
        crate::desktop::tool_panel::font::release(hwnd);
        SetWindowLongPtrW(hwnd, DWLP_USER, 0);
        return 0;
    }
    match msg {
        WM_INITDIALOG => {
            if let Err(error) = init(hwnd, dialog) {
                *dialog.error.borrow_mut() = Some(error);
                let _ = EndDialog(hwnd, 0);
            } else {
                #[cfg(test)]
                tests::initialized(hwnd);
            }
            1
        }
        WM_CLOSE => {
            let _ = EndDialog(hwnd, 0);
            1
        }
        WM_DPICHANGED if lp.0 != 0 => {
            let rect = *(lp.0 as *const RECT);
            if let Err(error) = crate::desktop::tool_panel::font::update(hwnd, wp.0 as u16 as u32) {
                *dialog.error.borrow_mut() = Some(error);
                let _ = EndDialog(hwnd, 0);
            } else {
                fonts(hwnd);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
            1
        }
        WM_COMMAND => {
            let id = wp.0 as u16;
            let code = (wp.0 >> 16) as u32;
            if id == IDCANCEL.0 as u16 {
                let _ = EndDialog(hwnd, 0);
            } else if id == IDOK.0 as u16 {
                dialog.accepted.set(true);
                let _ = EndDialog(hwnd, 1);
            } else if id == PORTS && code == LBN_SELCHANGE {
                refresh(hwnd, dialog);
            } else if id == MODE && code == CBN_SELCHANGE {
                if let (Some(index), Some(mode)) = (
                    selected(hwnd, PORTS, LB_GETCURSEL),
                    selected(hwnd, MODE, CB_GETCURSEL),
                ) {
                    let valid = dialog.model.borrow_mut().edit(index, mode).is_ok();
                    let _ = SetWindowTextW(
                        child(hwnd, STATUS),
                        &HSTRING::from(if valid {
                            ""
                        } else {
                            "显示设置超出限制，未修改"
                        }),
                    );
                    refresh(hwnd, dialog);
                }
            }
            1
        }
        WM_ERASEBKGND => {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            theme::fill(HDC(wp.0 as *mut _), &rect, theme::PANEL);
            1
        }
        WM_DRAWITEM if lp.0 != 0 => {
            theme::button(&*(lp.0 as *const DRAWITEMSTRUCT));
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            theme::colors(msg, wp, lp, false).0
        }
        _ => 0,
    }
}
#[cfg(test)]
#[path = "port_visibility_dialog_tests.rs"]
mod tests;
