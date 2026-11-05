//! 同步 owned 目录选择窗；真实 EDIT/LISTBOX，IME 优先，DPI 不重建草稿，退出后才交付意图。
use super::*;
use std::cell::{Cell, RefCell};
use windows::{
    core::{Error, HSTRING},
    Win32::UI::{
        Controls::EM_SETLIMITTEXT,
        HiDpi::*,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
    },
};
#[path = "art_picker_template.rs"]
mod template;
#[cfg(test)]
#[path = "art_picker_dialog_tests.rs"]
pub(in crate::desktop) mod tests;
use desktop::document_window::parameter_panel::theme;
const SEARCH: i32 = 1001;
const LIST: i32 = 1002;
const STATUS: i32 = 1003;
const REFRESH: i32 = 1004;
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
}
/// 同步显示借用；仅复用目录 UI，不给 Live 构造假的文档 State 或执行授权。
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
    SendMessageW(list, LB_RESETCONTENT, None, None);
    if let Some(catalog) = &ui.context.catalog {
        for index in model::search(Some(catalog), &query) {
            let Some(art) = catalog.handshake.capability(index) else {
                continue;
            };
            let suffix = if !art.enabled {
                " · 已禁用"
            } else if !(ui.context.allowed)(index) {
                " · 需选择 Loom 服务"
            } else {
                ""
            };
            let label = HSTRING::from(format!("{} · {}{}", art.label, art.id, suffix));
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
    update(hwnd, ui);
}
unsafe fn update(hwnd: HWND, ui: &Ui<'_>) {
    let enabled = selected(hwnd).is_some_and(ui.context.allowed);
    let _ = EnableWindow(GetDlgItem(Some(hwnd), IDOK.0).unwrap_or_default(), enabled);
    let status = if ui.context.loading {
        "正在加载节点能力…"
    } else if ui.context.failed {
        "加载失败；可刷新重试"
    } else if ui.context.catalog.is_none() {
        "节点能力尚未加载"
    } else if selected(hwnd).is_none() {
        "没有匹配的 Art"
    } else {
        ""
    };
    let status = if status.is_empty() {
        ui.context
            .origin
            .as_ref()
            .map(|origin| format!("本地 Art · {origin}"))
            .unwrap_or_default()
    } else {
        status.into()
    };
    let _ = SetWindowTextW(
        GetDlgItem(Some(hwnd), STATUS).unwrap_or_default(),
        &HSTRING::from(status),
    );
}
unsafe fn layout(hwnd: HWND) -> Result<()> {
    let dpi = GetDpiForWindow(hwnd).max(96);
    desktop::tool_panel::font::update(hwnd, dpi)?;
    let font = desktop::tool_panel::font::object(hwnd);
    let px = |value: i32| (i64::from(value) * i64::from(dpi) / 96) as i32;
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    let width = rect.right.max(px(240));
    let height = rect.bottom.max(px(180));
    for (id, x, y, w, h) in [
        (SEARCH, px(12), px(12), width - px(24), px(28)),
        (LIST, px(12), px(48), width - px(24), height - px(132)),
        (STATUS, px(12), height - px(78), width - px(24), px(26)),
        (REFRESH, px(12), height - px(42), px(94), px(28)),
        (IDOK.0, width - px(182), height - px(42), px(78), px(28)),
        (IDCANCEL.0, width - px(90), height - px(42), px(78), px(28)),
    ] {
        let control = GetDlgItem(Some(hwnd), id)?;
        SendMessageW(
            control,
            WM_SETFONT,
            Some(WPARAM(font.0 as usize)),
            Some(LPARAM(1)),
        );
        SetWindowPos(control, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE)?;
    }
    Ok(())
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
            for id in [SEARCH, LIST, REFRESH, IDOK.0, IDCANCEL.0] {
                SetWindowSubclass(
                    GetDlgItem(Some(hwnd), id)?,
                    Some(policy),
                    1,
                    pointer as usize,
                )
                .ok()?;
            }
            layout(hwnd)?;
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
    // DialogBox 自带嵌套消息泵；复用原会话输入门，而不是依赖外层 Application 的过滤。
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
        DefSubclassProc(hwnd, msg, wp, lp)
    })
    .unwrap_or(LRESULT(0))
}
fn blocked(ui: &Ui<'_>, msg: u32) -> bool {
    desktop::session_end::with_owner(ui.context.session_end, || {
        desktop::session_end::blocks_input(msg)
    })
}
