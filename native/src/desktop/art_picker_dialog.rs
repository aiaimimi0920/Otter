//! Synchronous owned catalog picker: real EDIT/LISTBOX, IME precedence, no draft recreation on DPI changes, and intent delivered only after exit.
use super::*;
use std::cell::{Cell, RefCell};
use windows::{
    core::{w, Error, HSTRING},
    Win32::UI::{
        Controls::{EM_SETCUEBANNER, EM_SETLIMITTEXT},
        HiDpi::*,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
    },
};
#[path = "art_picker_template.rs"]
mod template;
#[cfg(test)]
#[path = "art_picker_dialog_tests.rs"]
pub(in crate::desktop) mod tests;
#[path = "art_picker_view.rs"]
mod view;
use desktop::document_window::parameter_panel::theme;
const SEARCH: i32 = 1001;
const LIST: i32 = 1002;
const STATUS: i32 = 1003;
const REFRESH: i32 = 1004;
const CLEAR: i32 = 1005;
const USER: WINDOW_LONG_PTR_INDEX =
    WINDOW_LONG_PTR_INDEX((2 * std::mem::size_of::<isize>()) as i32);
pub(in crate::desktop) enum Outcome {
    Cancel,
    Refresh,
    Select(usize),
}
struct Ui<'a> {
    context: Context<'a>,
    result: RefCell<Outcome>,
    error: RefCell<Option<Error>>,
    composing: Cell<u8>,
    pressed: Cell<Option<(usize, POINT)>>,
}
/// Borrow for synchronous display; reuse only catalog UI, without fabricating document State or execution authorization for Live.
pub(in crate::desktop) struct Context<'a> {
    pub catalog: Option<Rc<Loaded>>,
    pub origin: Option<String>,
    pub allowed: &'a dyn Fn(usize) -> bool,
    pub session_end: &'a Rc<desktop::session_end::Control>,
    pub protected: bool,
    pub loading: bool,
    pub failed: bool,
}
pub(super) unsafe fn show(
    owner: HWND,
    state: &State,
    snapshot: &model::Snapshot,
) -> Result<Outcome> {
    let catalog = state.catalog.borrow();
    let (loading, failed) = (catalog.job.is_busy() || catalog.requested, catalog.failed);
    drop(catalog);
    show_with(
        owner,
        Context {
            catalog: snapshot.catalog.clone(),
            origin: snapshot.origin.clone(),
            allowed: &|index| snapshot.route(index, state).is_some(),
            session_end: &state.session_end,
            protected: state.capture_protection_requested.get(),
            loading,
            failed,
        },
    )
}
pub(in crate::desktop) unsafe fn show_with(owner: HWND, context: Context<'_>) -> Result<Outcome> {
    let ui = Ui {
        context,
        result: RefCell::new(Outcome::Cancel),
        error: RefCell::new(None),
        composing: Cell::new(0),
        pressed: Cell::new(None),
    };
    let template = template::template();
    let result = DialogBoxIndirectParamW(
        None,
        template.as_ptr().cast(),
        Some(owner),
        Some(procedure),
        LPARAM(&ui as *const _ as isize),
    );
    if let Some(error) = ui.error.into_inner() {
        return Err(error);
    }
    if result == -1 {
        return Err(Error::from_win32());
    }
    Ok(ui.result.into_inner())
}
unsafe fn selected(hwnd: HWND) -> Option<usize> {
    let list = GetDlgItem(Some(hwnd), LIST).ok()?;
    let row = SendMessageW(list, LB_GETCURSEL, None, None).0;
    if row < 0 {
        return None;
    }
    usize::try_from(SendMessageW(list, LB_GETITEMDATA, Some(WPARAM(row as usize)), None).0).ok()
}
unsafe fn text(hwnd: HWND) -> String {
    let mut text = [0u16; 257];
    let count = GetWindowTextW(hwnd, &mut text).max(0) as usize;
    String::from_utf16_lossy(&text[..count])
}
unsafe fn refresh(hwnd: HWND, ui: &Ui<'_>) {
    let list = GetDlgItem(Some(hwnd), LIST).unwrap_or_default();
    let query = text(GetDlgItem(Some(hwnd), SEARCH).unwrap_or_default());
    let _redraw = neuro_mini_ui::paint_batch::Hold::new(hwnd);
    let list_redraw = neuro_mini_ui::paint_batch::Hold::new(list);
    ui.pressed.set(None);
    SendMessageW(list, LB_RESETCONTENT, None, None);
    if let Some(catalog) = &ui.context.catalog {
        for index in model::search(Some(catalog), &query) {
            let Some(art) = catalog.handshake.capability(index) else {
                continue;
            };
            let suffix = if !art.enabled {
                crate::ui_text::text("art-picker-disabled")
            } else if !(ui.context.allowed)(index) {
                crate::ui_text::text("art-picker-service-required")
            } else {
                ""
            };
            let label = HSTRING::from(format!("{}{}", art.label, suffix));
            let row = SendMessageW(
                list,
                LB_ADDSTRING,
                None,
                Some(LPARAM(label.as_ptr() as isize)),
            )
            .0;
            if row >= 0 {
                SendMessageW(
                    list,
                    LB_SETITEMDATA,
                    Some(WPARAM(row as usize)),
                    Some(LPARAM(index as isize)),
                );
            }
        }
    }
    SendMessageW(list, LB_SETCURSEL, Some(WPARAM(0)), None);
    drop(list_redraw);
    update(hwnd, ui);
}
unsafe fn update(hwnd: HWND, ui: &Ui<'_>) {
    view::visible(
        hwnd,
        CLEAR,
        !text(GetDlgItem(Some(hwnd), SEARCH).unwrap_or_default()).is_empty(),
    );
    let status = if ui.context.loading {
        crate::ui_text::text("art-picker-loading")
    } else if ui.context.failed {
        crate::ui_text::text("art-picker-failed")
    } else if ui.context.catalog.is_none() {
        crate::ui_text::text("art-picker-not-loaded")
    } else if selected(hwnd).is_none() {
        crate::ui_text::text("art-picker-empty")
    } else {
        ""
    };
    let empty = SendMessageW(
        GetDlgItem(Some(hwnd), LIST).unwrap_or_default(),
        LB_GETCOUNT,
        None,
        None,
    )
    .0 == 0;
    view::visible(hwnd, LIST, !empty);
    view::visible(hwnd, STATUS, empty);
    view::visible(hwnd, REFRESH, empty && !ui.context.loading);
    let _ = SetWindowTextW(
        GetDlgItem(Some(hwnd), STATUS).unwrap_or_default(),
        &HSTRING::from(status),
    );
}
unsafe fn layout(hwnd: HWND) -> Result<()> {
    view::layout(hwnd)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    std::panic::catch_unwind(|| dispatch(hwnd, msg, wp, lp)).unwrap_or_else(|_| {
        let _ = EndDialog(hwnd, 0);
        1
    })
}
unsafe fn dispatch(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        SetWindowLongPtrW(hwnd, USER, lp.0);
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, USER, 0);
        desktop::tool_panel::font::release(hwnd);
        return 0;
    }
    let pointer = GetWindowLongPtrW(hwnd, USER) as *const Ui<'_>;
    if pointer.is_null() {
        return 0;
    }
    let ui = &*pointer;
    if msg == WM_INITDIALOG {
        let initialized = (|| -> Result<()> {
            SetWindowDisplayAffinity(
                hwnd,
                if ui.context.protected {
                    WDA_EXCLUDEFROMCAPTURE
                } else {
                    WDA_NONE
                },
            )?;
            SendMessageW(
                GetDlgItem(Some(hwnd), SEARCH)?,
                EM_SETLIMITTEXT,
                Some(WPARAM(256)),
                None,
            );
            SendMessageW(
                GetDlgItem(Some(hwnd), SEARCH)?,
                EM_SETCUEBANNER,
                Some(WPARAM(1)),
                Some(LPARAM(
                    HSTRING::from(crate::ui_text::text("art-picker-search")).as_ptr() as isize,
                )),
            );
            for id in [SEARCH, LIST, REFRESH, CLEAR, IDCANCEL.0] {
                SetWindowSubclass(
                    GetDlgItem(Some(hwnd), id)?,
                    Some(policy),
                    1,
                    pointer as usize,
                )
                .ok()?;
            }
            for id in [REFRESH, CLEAR, IDCANCEL.0] {
                neuro_mini_ui::prepare_button(GetDlgItem(Some(hwnd), id)?)?;
            }
            view::position(hwnd, GetWindow(hwnd, GW_OWNER)?)?;
            refresh(hwnd, ui);
            Ok(())
        })();
        if let Err(error) = initialized {
            *ui.error.borrow_mut() = Some(error);
            let _ = EndDialog(hwnd, 0);
        }
        #[cfg(test)]
        {
            let ready = ui.error.borrow().is_none();
            if ready {
                tests::initialized(hwnd);
            }
        }
        return 1;
    }
    if let Some(result) = view::paint(hwnd, msg, wp, lp, ui) {
        return result;
    }
    if msg == WM_DPICHANGED {
        let rect = &*(lp.0 as *const RECT);
        let result = SetWindowPos(
            hwnd,
            None,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .and_then(|_| layout(hwnd));
        if let Err(error) = result {
            *ui.error.borrow_mut() = Some(error);
            let _ = EndDialog(hwnd, 0);
        }
        return 1;
    }
    if matches!(
        msg,
        WM_CTLCOLORDLG | WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC | WM_CTLCOLORLISTBOX
    ) {
        return theme::colors(msg, wp, lp, false).0;
    }
    // DialogBox has its own nested message pump; reuse the original session input gate instead of relying on outer Application filtering.
    if blocked(ui, msg) {
        return 1;
    }
    if msg == WM_COMMAND {
        let id = (wp.0 & 0xffff) as i32;
        let notification = (wp.0 >> 16) as u32;
        if id == SEARCH && notification == EN_CHANGE {
            refresh(hwnd, ui);
            return 1;
        }
        if id == LIST && notification == LBN_SELCHANGE {
            update(hwnd, ui);
            return 1;
        }
        if id == CLEAR {
            let edit = GetDlgItem(Some(hwnd), SEARCH).unwrap_or_default();
            let _ = SetWindowTextW(edit, w!(""));
            let _ = SetFocus(Some(edit));
            return 1;
        }
        if id == IDCANCEL.0 {
            if ui.composing.get() == 0 || lp.0 != 0 {
                let _ = EndDialog(hwnd, 0);
            }
            return 1;
        }
        if id == REFRESH {
            *ui.result.borrow_mut() = Outcome::Refresh;
            let _ = EndDialog(hwnd, 1);
            return 1;
        }
        if id == IDOK.0 || id == LIST && notification == LBN_DBLCLK {
            if ui.composing.get() == 0 {
                if let Some(index) = selected(hwnd).filter(|index| (ui.context.allowed)(*index)) {
                    *ui.result.borrow_mut() = Outcome::Select(index);
                    let _ = EndDialog(hwnd, 1);
                }
            }
            return 1;
        }
    }
    if msg == WM_CLOSE {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    if msg == WM_ACTIVATE
        && wp.0 & 0xffff == WA_INACTIVE as usize
        && IsWindowVisible(hwnd).as_bool()
    {
        let _ = EndDialog(hwnd, 0);
        return 1;
    }
    0
}
unsafe extern "system" fn policy(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    std::panic::catch_unwind(|| {
        if msg == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(hwnd, Some(policy), 1);
            return DefSubclassProc(hwnd, msg, wp, lp);
        }
        let ui = &*(data as *const Ui<'_>);
        if blocked(ui, msg) {
            return LRESULT(0);
        }
        if msg == WM_IME_STARTCOMPOSITION {
            ui.composing.set(1);
        }
        if msg == WM_IME_ENDCOMPOSITION {
            ui.composing.set(2);
        }
        if msg == WM_KEYUP || msg == WM_KEYDOWN && lp.0 & (1 << 30) == 0 {
            ui.composing.set(ui.composing.get() & 1);
        }
        if ui.composing.get() == 0 && key(msg, wp) {
            if lp.0 & (1 << 30) == 0 {
                let _ = EndDialog(GetParent(hwnd).unwrap_or_default(), 0);
            }
            return LRESULT(0);
        }
        if ui.composing.get() == 0
            && msg == WM_KEYDOWN
            && wp.0 == VK_RETURN.0 as usize
            && [SEARCH, LIST].contains(&GetDlgCtrlID(hwnd))
        {
            if lp.0 & (1 << 30) == 0 {
                SendMessageW(
                    GetParent(hwnd).unwrap_or_default(),
                    WM_COMMAND,
                    Some(WPARAM(IDOK.0 as usize)),
                    None,
                );
            }
            return LRESULT(0);
        }
        if GetDlgCtrlID(hwnd) == LIST {
            let point = POINT {
                x: lp.0 as i16 as i32,
                y: (lp.0 >> 16) as i16 as i32,
            };
            if msg == WM_LBUTTONDOWN {
                ui.pressed.set(row_at(hwnd, lp).map(|row| (row, point)));
            } else if matches!(msg, WM_CANCELMODE | WM_CAPTURECHANGED) {
                ui.pressed.set(None);
            } else if msg == WM_LBUTTONUP {
                let press = ui.pressed.take();
                let result = DefSubclassProc(hwnd, msg, wp, lp);
                let row = row_at(hwnd, lp);
                let threshold = (4 * GetDpiForWindow(hwnd).max(96) / 96).max(1);
                if ui.composing.get() == 0
                    && press.is_some_and(|(old, start)| {
                        row == Some(old)
                            && start.x.abs_diff(point.x) <= threshold
                            && start.y.abs_diff(point.y) <= threshold
                    })
                {
                    SendMessageW(
                        GetParent(hwnd).unwrap_or_default(),
                        WM_COMMAND,
                        Some(WPARAM(IDOK.0 as usize)),
                        None,
                    );
                }
                return result;
            }
        }
        DefSubclassProc(hwnd, msg, wp, lp)
    })
    .unwrap_or(LRESULT(0))
}
unsafe fn row_at(hwnd: HWND, lp: LPARAM) -> Option<usize> {
    let row = SendMessageW(hwnd, LB_ITEMFROMPOINT, None, Some(lp)).0 as usize;
    if row >> 16 != 0 {
        return None;
    }
    let mut rect = RECT::default();
    let result = SendMessageW(
        hwnd,
        LB_GETITEMRECT,
        Some(WPARAM(row)),
        Some(LPARAM(&mut rect as *mut _ as isize)),
    );
    let point = POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    (result.0 != LB_ERR as isize && windows::Win32::Graphics::Gdi::PtInRect(&rect, point).as_bool())
        .then_some(row)
}
fn blocked(ui: &Ui<'_>, msg: u32) -> bool {
    desktop::session_end::with_owner(ui.context.session_end, || {
        desktop::session_end::blocks_input(msg)
    })
}
