//! 真实同步模态窗的首显前控件回归；仅在未切换 desktop 内运行，不注入全局输入。
use super::*;
use std::any::Any;
use windows::{core::w, Win32::Graphics::Gdi::*};
type Initialize = Box<dyn FnOnce(HWND)>;
thread_local! {
    static READY: RefCell<Option<Initialize>> = const { RefCell::new(None) };
    static PANIC: RefCell<Option<Box<dyn Any + Send>>> = const { RefCell::new(None) };
}
pub(super) unsafe fn initialized(hwnd: HWND) {
    let action = READY.with(|slot| slot.borrow_mut().take());
    if let Some(action) = action {
        if let Err(error) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action(hwnd)))
        {
            PANIC.with(|slot| *slot.borrow_mut() = Some(error));
        }
        let _ = EndDialog(hwnd, 0);
    }
}
pub(in crate::desktop) unsafe fn live_check(owner: HWND) {
    READY.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(move |hwnd| {
            assert_eq!(GetWindow(hwnd, GW_OWNER).unwrap(), owner);
            assert!(!IsWindowVisible(hwnd).as_bool());
            let mut affinity = 0;
            GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            let edit = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            SetWindowTextW(edit, w!("TEST-ART")).unwrap();
            assert_eq!(selected(hwnd), Some(0));
            SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
            command(hwnd, IDOK.0);
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
            SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
            SendMessageW(edit, WM_KEYUP, Some(WPARAM(VK_RETURN.0 as usize)), None);
            command(hwnd, IDOK.0);
        }))
    });
}
pub(in crate::desktop) fn check_hook() {
    let pending = READY.with(|slot| slot.borrow_mut().take());
    if let Some(error) = PANIC.with(|slot| slot.borrow_mut().take()) {
        std::panic::resume_unwind(error);
    }
    assert!(
        pending.is_none(),
        "Live dialog initialization hook was not called"
    );
}
unsafe fn run(action: impl FnOnce(HWND) + 'static) -> Outcome {
    let _apartment = desktop::Apartment::new().unwrap();
    let (_, doc) = super::super::tests::document();
    doc.state.catalog.borrow_mut().loaded = Some(super::super::tests::catalog(|raw| {
        let mut second = raw["capabilities"]["artDefinitions"][0].clone();
        second["id"] = "disabled".into();
        second["label"] = "第二项".into();
        second["enabled"] = false.into();
        raw["capabilities"]["artDefinitions"]
            .as_array_mut()
            .unwrap()
            .push(second);
    }));
    doc.state.capture_protection_requested.set(true);
    let snapshot = model::Snapshot::capture(doc.window.0, &doc.state);
    READY.with(|slot| *slot.borrow_mut() = Some(Box::new(action)));
    let result = show(doc.window.0, &doc.state, &snapshot);
    let pending = READY.with(|slot| slot.borrow_mut().take());
    if let Some(error) = PANIC.with(|slot| slot.borrow_mut().take()) {
        std::panic::resume_unwind(error);
    }
    assert!(
        pending.is_none(),
        "dialog initialization hook was not called"
    );
    result.unwrap()
}
// 调用者限于同步 initialized 回调；返回借用不得越过 show 的栈上 Ui 生命周期。
unsafe fn ui<'a>(hwnd: HWND) -> &'a Ui<'a> {
    &*(GetWindowLongPtrW(hwnd, USER) as *const Ui<'_>)
}
unsafe fn command(hwnd: HWND, id: i32) {
    SendMessageW(hwnd, WM_COMMAND, Some(WPARAM(id as usize)), None);
}
#[test]
fn actual_search_list_preserves_catalog_index_and_disabled_item_cannot_add() {
    unsafe {
        let result = run(|hwnd| {
            let search = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            let list = GetDlgItem(Some(hwnd), LIST).unwrap();
            assert_eq!(SendMessageW(list, LB_GETCOUNT, None, None).0, 2);
            SetWindowTextW(search, w!("第二")).unwrap();
            assert_eq!(SendMessageW(list, LB_GETCOUNT, None, None).0, 1);
            assert_eq!(selected(hwnd), Some(1));
            assert!(!(ui(hwnd).context.allowed)(selected(hwnd).unwrap()));
            command(hwnd, IDOK.0);
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
            SetWindowTextW(search, w!("TEST-ART")).unwrap();
            assert_eq!(selected(hwnd), Some(0));
            assert!((ui(hwnd).context.allowed)(selected(hwnd).unwrap()));
            command(hwnd, IDOK.0);
        });
        assert!(matches!(result, Outcome::Select(0)));
    }
}
#[test]
fn ime_edges_hold_default_add_and_keyboard_cancel_until_key_release() {
    unsafe {
        let result = run(|hwnd| {
            let search = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            SendMessageW(search, WM_IME_STARTCOMPOSITION, None, None);
            command(hwnd, IDOK.0);
            command(hwnd, IDCANCEL.0);
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
            assert_eq!(ui(hwnd).composing.get(), 1);
            SendMessageW(search, WM_IME_ENDCOMPOSITION, None, None);
            command(hwnd, IDOK.0);
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
            assert_eq!(ui(hwnd).composing.get(), 2);
            SendMessageW(search, WM_KEYUP, Some(WPARAM(VK_RETURN.0 as usize)), None);
            assert_eq!(ui(hwnd).composing.get(), 0);
            command(hwnd, IDOK.0);
        });
        assert!(matches!(result, Outcome::Select(0)));
    }
}
#[test]
fn dpi_keeps_edit_and_draft_and_first_show_is_capture_excluded_then_font_is_released() {
    unsafe {
        let handles = Rc::new(RefCell::new(None));
        let capture = handles.clone();
        run(move |hwnd| {
            assert!(!IsWindowVisible(hwnd).as_bool());
            let mut affinity = 0;
            GetWindowDisplayAffinity(hwnd, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
            let edit = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            SetWindowTextW(edit, w!("中文😀")).unwrap();
            SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).unwrap();
            SendMessageW(
                hwnd,
                WM_DPICHANGED,
                Some(WPARAM(144 | (144 << 16))),
                Some(LPARAM(&rect as *const _ as isize)),
            );
            assert_eq!(GetDlgItem(Some(hwnd), SEARCH).unwrap(), edit);
            assert_eq!(text(edit), "中文😀");
            assert_eq!(ui(hwnd).composing.get(), 1);
            *capture.borrow_mut() = Some((hwnd, desktop::tool_panel::font::object(hwnd)));
        });
        let (hwnd, font) = handles.borrow().unwrap();
        assert!(!IsWindow(Some(hwnd)).as_bool());
        let mut descriptor = LOGFONTW::default();
        assert_eq!(
            GetObjectW(
                font,
                std::mem::size_of::<LOGFONTW>() as i32,
                Some((&mut descriptor as *mut LOGFONTW).cast())
            ),
            0
        );
    }
}
#[test]
fn nested_dialog_pump_freezes_search_and_commands_during_session_pause_then_resumes() {
    unsafe {
        let result = run(|hwnd| {
            let control = ui(hwnd).context.session_end;
            let edit = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            SetWindowTextW(edit, w!("test")).unwrap();
            desktop::session_end::with_query(
                control,
                || true,
                || {
                    assert_eq!(
                        SendMessageW(
                            GetWindow(hwnd, GW_OWNER).unwrap(),
                            WM_QUERYENDSESSION,
                            None,
                            None
                        ),
                        LRESULT(1)
                    );
                },
            );
            SetWindowTextW(edit, w!("changed")).unwrap();
            assert_eq!(text(edit), "test");
            command(hwnd, IDOK.0);
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
            SendMessageW(
                GetWindow(hwnd, GW_OWNER).unwrap(),
                WM_ENDSESSION,
                Some(WPARAM(0)),
                None,
            );
            assert!(!control.paused());
            command(hwnd, IDOK.0);
        });
        assert!(matches!(result, Outcome::Select(0)));
    }
}

#[test]
fn compact_menu_has_no_caption_footer_and_clear_preserves_search_window() {
    unsafe {
        run(|hwnd| {
            assert_eq!(GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_CAPTION.0, 0);
            assert!(GetDlgItem(Some(hwnd), IDOK.0).is_err());
            let mut bounds = RECT::default();
            GetWindowRect(hwnd, &mut bounds).unwrap();
            let dpi = GetDpiForWindow(hwnd).max(96) as i32;
            assert_eq!(bounds.right - bounds.left, 250 * dpi / 96);
            assert_eq!(bounds.bottom - bounds.top, 300 * dpi / 96);
            let search = GetDlgItem(Some(hwnd), SEARCH).unwrap();
            SetWindowTextW(search, w!("  TEST-ART  ")).unwrap();
            assert_eq!(selected(hwnd), Some(0));
            SetWindowTextW(search, w!("no such art")).unwrap();
            let list = GetDlgItem(Some(hwnd), LIST).unwrap();
            assert_eq!(GetWindowLongW(list, GWL_STYLE) as u32 & WS_VISIBLE.0, 0);
            assert_ne!(
                GetWindowLongW(GetDlgItem(Some(hwnd), STATUS).unwrap(), GWL_STYLE) as u32
                    & WS_VISIBLE.0,
                0
            );
            command(hwnd, CLEAR);
            assert_eq!(GetDlgItem(Some(hwnd), SEARCH).unwrap(), search);
            assert_eq!(text(search), "");
            assert_eq!(
                SendMessageW(
                    GetDlgItem(Some(hwnd), LIST).unwrap(),
                    LB_GETCOUNT,
                    None,
                    None
                )
                .0,
                2
            );
            assert!(
                row_at(list, LPARAM((150 << 16) | 8)).is_none(),
                "blank list area is not an Art row"
            );
            assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
        });
    }
}

#[test]
fn list_keyboard_selection_does_not_add_and_single_click_requires_same_row_and_short_motion() {
    unsafe {
        for moved in [true, false] {
            let result = run(move |hwnd| {
                let list = GetDlgItem(Some(hwnd), LIST).unwrap();
                SendMessageW(
                    hwnd,
                    WM_COMMAND,
                    Some(WPARAM(LIST as usize | (LBN_SELCHANGE as usize) << 16)),
                    Some(LPARAM(list.0 as isize)),
                );
                assert!(matches!(*ui(hwnd).result.borrow(), Outcome::Cancel));
                let point = LPARAM((8 << 16) | 8);
                SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
                SendMessageW(
                    list,
                    WM_LBUTTONUP,
                    None,
                    Some(if moved { LPARAM((8 << 16) | 20) } else { point }),
                );
            });
            assert!(if moved {
                matches!(result, Outcome::Cancel)
            } else {
                matches!(result, Outcome::Select(0))
            });
        }
    }
}
