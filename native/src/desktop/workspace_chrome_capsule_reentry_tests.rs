//! Synchronous child messages cannot revive closed owners, overwrite successor focus, or interpret old indices with new catalogs.
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
struct Replace<'a> {
    ui: &'a Ui,
    port: Port,
    message: u32,
    once: Cell<bool>,
    close: bool,
}
unsafe extern "system" fn replace(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let target = &*(data as *const Replace<'_>);
    if msg == target.message && !target.once.replace(true) {
        if target.close {
            let parent = GetParent(hwnd).unwrap();
            SendMessageW(parent, WM_CLOSE, None, None);
        }
        request(target.ui, target.port, true);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn synchronous_focus_request_keeps_the_successor_and_closed_owner_is_not_restored() {
    unsafe {
        for close in [false, true] {
            let mut app = app();
            let hwnd = open(&mut app, "text", false);
            let ui = app.chrome.ui.clone();
            let target = Replace {
                ui: &ui,
                port: port(&app, "n"),
                message: WM_SETFOCUS,
                once: Cell::new(false),
                close,
            };
            SetWindowSubclass(
                edit(&app, "text"),
                Some(replace),
                598,
                &target as *const _ as usize,
            )
            .ok()
            .unwrap();
            request(&ui, port(&app, "text"), true);
            app.sync_workspace_chrome(true);
            assert!(target.once.get());
            assert_eq!(ui.rail_focus.borrow().as_ref().unwrap().port, target.port);
            if close {
                assert!(panel(&app).retired());
            }
            tick(&mut app);
            assert_eq!(GetFocus(), edit(&app, "n"));
            assert!(!panel(&app).retired());
            if !close {
                assert_eq!(panel(&app).handle(), hwnd);
            }
            app.shutdown();
        }
    }
}
#[test]
fn synchronous_placement_request_is_preserved_without_borrowing_the_rail_owner() {
    unsafe {
        let mut app = app();
        let hwnd = open(&mut app, "text", false);
        let ui = app.chrome.ui.clone();
        let target = Replace {
            ui: &ui,
            port: port(&app, "n"),
            message: WM_WINDOWPOSCHANGED,
            once: Cell::new(false),
            close: false,
        };
        SetWindowSubclass(hwnd, Some(replace), 598, &target as *const _ as usize)
            .ok()
            .unwrap();
        SetWindowPos(hwnd, None, 0, 0, 1, 1, SWP_NOACTIVATE | SWP_NOZORDER).unwrap();
        assert!(target.once.get());
        tick(&mut app);
        assert_eq!(panel(&app).handle(), hwnd);
        assert_eq!(GetFocus(), edit(&app, "n"));
        app.shutdown();
    }
}
#[test]
fn catalog_replacement_cannot_reinterpret_a_queued_field_focus() {
    unsafe {
        let mut app = app();
        assert!(request(&app.chrome.ui, port(&app, "text"), true));
        app.documents[0]
            .state
            .selected_art
            .borrow_mut()
            .as_mut()
            .unwrap()
            .parameters
            .set("n", Some(serde_json::json!(7)))
            .unwrap();
        tick(&mut app);
        assert_eq!(GetFocus(), app.documents[0].window.0);
        assert!(app.chrome.ui.rail_focus.borrow().is_none());
        app.shutdown();
    }
}
