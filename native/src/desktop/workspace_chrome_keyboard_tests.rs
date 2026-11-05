//! 私有desktop自有HWND和线程键表；无全局激活、SendInput或用户窗口操作。
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};

struct Keys([u8; 256]);
impl Keys {
    unsafe fn held(keys: &[VIRTUAL_KEY]) -> Self {
        let mut old = [0; 256];
        GetKeyboardState(&mut old).unwrap();
        let mut next = [0; 256];
        for key in keys {
            next[key.0 as usize] = 0x80;
        }
        SetKeyboardState(&next).unwrap();
        Self(old)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            SetKeyboardState(&self.0).unwrap();
        }
    }
}
unsafe fn app() -> Application {
    let mut app = super::super::tests::app();
    app.add_window(false).unwrap();
    for (i, doc) in app.documents.iter().enumerate() {
        SetWindowPos(
            doc.window.0,
            None,
            240 + i as i32 * 400,
            180,
            160,
            120,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    }
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    let _ = SetFocus(Some(app.documents[0].window.0));
    assert_eq!(GetFocus(), app.documents[0].window.0);
    app
}
unsafe fn key(app: &Application, code: VIRTUAL_KEY, held: &[VIRTUAL_KEY], repeat: bool) -> bool {
    let _keys = Keys::held(held);
    app.translate_workspace_chrome(&MSG {
        hwnd: GetFocus(),
        message: WM_KEYDOWN,
        wParam: WPARAM(code.0 as usize),
        lParam: LPARAM(if repeat { 1 << 30 } else { 0 }),
        ..Default::default()
    })
}
unsafe fn up(app: &Application, code: VIRTUAL_KEY) -> bool {
    app.translate_workspace_chrome(&MSG {
        hwnd: GetFocus(),
        message: WM_KEYUP,
        wParam: WPARAM(code.0 as usize),
        ..Default::default()
    })
}
fn at(node: usize, side: Side) -> Port {
    Port {
        node,
        side,
        index: 0,
    }
}

#[test]
fn f6_navigation_wraps_visible_owned_strips_and_repeat_only_moves_navigation() {
    unsafe {
        let mut app = app();
        assert!(key(&app, VK_F6, &[], false));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Input)));
        assert!(up(&app, VK_F6));
        assert!(key(&app, VK_F6, &[], true));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Input)));
        assert!(key(&app, VK_TAB, &[VK_SHIFT], false));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(1, Side::Output)));
        assert!(key(&app, VK_HOME, &[], false));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Input)));
        assert!(key(&app, VK_RIGHT, &[], true));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Output)));
        assert!(key(&app, VK_END, &[], false));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(1, Side::Output)));
        app.shutdown();
    }
}
#[test]
fn enter_space_disclose_once_and_escape_keyup_never_deletes_the_origin() {
    unsafe {
        let mut app = app();
        let host = app.documents[0].window.0;
        key(&app, VK_F6, &[], false);
        let port = at(0, Side::Input);
        assert!(key(&app, VK_RETURN, &[], false));
        assert!(app.chrome.ui.disclosed.borrow().contains(&port));
        assert!(key(&app, VK_RETURN, &[], true));
        assert!(app.chrome.ui.disclosed.borrow().contains(&port));
        assert!(key(&app, VK_SPACE, &[], false));
        assert!(!app.chrome.ui.disclosed.borrow().contains(&port));
        assert!(key(&app, VK_ESCAPE, &[], false));
        assert_eq!(GetFocus(), host);
        assert!(key(&app, VK_ESCAPE, &[], true));
        assert!(up(&app, VK_ESCAPE));
        assert!(app.documents[0].is_alive());
        assert!(app.chrome.ui.keyboard.focused().is_none());
        app.shutdown();
    }
}
#[test]
fn ctrl_enter_queues_one_link_through_original_transaction_and_ignores_repeat() {
    unsafe {
        let mut app = app();
        key(&app, VK_F6, &[VK_SHIFT], false);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Output)));
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        assert_eq!(app.chrome.ui.keyboard.source(), Some(at(0, Side::Output)));
        assert!(!app.chrome.idle());
        key(&app, VK_TAB, &[], false);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(1, Side::Input)));
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        assert!(app.chrome.ui.pending.borrow().is_some());
        assert!(app.chrome.ui.keyboard.source().is_none());
        assert!(key(&app, VK_RETURN, &[VK_CONTROL], true));
        app.pending_link_changes();
        assert_eq!(app.workspace.links().len(), 1);
        assert_eq!(
            app.workspace.links()[0].from_unit,
            app.documents[0].state.unit_id
        );
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn reconnect_escape_is_two_stage_and_old_edge_survives_cancel_and_lock_before_commit() {
    unsafe {
        let mut app = app();
        let catalog = app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .unwrap()
            .catalog
            .clone();
        app.apply_link_creation(desktop::link_catalog::Creation {
            link: catalog.make_link(0, 1, 0, 0).unwrap().unwrap(),
            catalog,
        });
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let old = app.workspace.links()[0].clone();
        key(&app, VK_F6, &[], false);
        key(&app, VK_TAB, &[], false);
        key(&app, VK_TAB, &[], false);
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        assert_eq!(app.chrome.ui.keyboard.source(), Some(at(0, Side::Output)));
        key(&app, VK_ESCAPE, &[], false);
        assert!(app.chrome.ui.keyboard.focused().is_some());
        assert!(app.chrome.ui.keyboard.source().is_none());
        assert_eq!(app.workspace.links(), std::slice::from_ref(&old));
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        app.documents[1].state.group_locked.set(true);
        app.pending_link_changes();
        assert_eq!(app.workspace.links(), [old]);
        app.shutdown();
    }
}
#[test]
fn hidden_ports_locked_nodes_and_clean_mode_cannot_gain_keyboard_focus() {
    unsafe {
        let mut app = app();
        app.workspace
            .set_port_visibility(
                &app.documents[0].state.unit_id,
                crate::workspace::ports::Visibility::read(r#"{"image":false}"#).unwrap(),
            )
            .unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        key(&app, VK_F6, &[], false);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Output)));
        app.documents[1].state.group_locked.set(true);
        app.sync_workspace_chrome(true);
        key(&app, VK_TAB, &[], false);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(at(0, Side::Output)));
        app.chrome.ui.mode.set(Mode::None);
        window::refresh(&app.chrome.ui).unwrap();
        assert!(app.chrome.ui.keyboard.focused().is_none());
        app.shutdown();
    }
}
#[test]
fn stale_catalog_hidden_source_and_capture_pause_retire_keyboard_link_without_topology_changes() {
    unsafe {
        let mut app = app();
        for pause in [false, true] {
            key(&app, VK_F6, &[VK_SHIFT], false);
            key(&app, VK_RETURN, &[VK_CONTROL], false);
            key(&app, VK_TAB, &[], false);
            if pause {
                app.capture_active.set(true);
            } else {
                app.documents[0].state.group_hidden.set(true);
            }
            app.sync_workspace_chrome(true);
            assert!(app.chrome.ui.keyboard.focused().is_none());
            assert!(app.chrome.ui.keyboard.source().is_none());
            assert!(app.chrome.idle());
            assert!(app.workspace.links().is_empty());
            app.capture_active.set(false);
            app.documents[0].state.group_hidden.set(false);
            app.sync_workspace_chrome(true);
            let _ = SetFocus(Some(app.documents[0].window.0));
        }
        app.shutdown();
    }
}
#[test]
fn external_focus_and_host_destruction_retire_without_changing_foreground() {
    unsafe {
        let mut app = app();
        let foreground = GetForegroundWindow();
        key(&app, VK_F6, &[VK_SHIFT], false);
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        let _ = SetFocus(Some(app.documents[1].window.0));
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert!(app.chrome.idle());
        let _ = SetFocus(Some(app.documents[0].window.0));
        key(&app, VK_F6, &[], false);
        DestroyWindow(app.documents[0].window.0).unwrap();
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert_eq!(GetForegroundWindow(), foreground);
        app.shutdown();
    }
}
unsafe extern "system" fn retire_on_focus(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_SETFOCUS {
        (*(data as *const Ui)).keyboard.retire();
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn synchronous_focus_reentry_cannot_resurrect_retired_candidate() {
    unsafe {
        let mut app = app();
        let hwnd = strip(&app.chrome.ui, at(0, Side::Input)).unwrap();
        SetWindowSubclass(
            hwnd,
            Some(retire_on_focus),
            53,
            Rc::as_ptr(&app.chrome.ui) as usize,
        )
        .ok()
        .unwrap();
        assert!(key(&app, VK_F6, &[], false));
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert!(app.chrome.idle());
        RemoveWindowSubclass(hwnd, Some(retire_on_focus), 53)
            .ok()
            .unwrap();
        app.shutdown();
    }
}

#[test]
fn extra_modifiers_foreign_message_and_unselected_host_enter_do_not_change_state() {
    unsafe {
        let mut app = app();
        assert!(!key(&app, VK_RETURN, &[VK_CONTROL], false));
        for held in [&[VK_CONTROL][..], &[VK_MENU], &[VK_LWIN], &[VK_RWIN]] {
            assert!(!key(&app, VK_F6, held, false));
        }
        assert!(app.chrome.ui.keyboard.focused().is_none());
        key(&app, VK_F6, &[], false);
        let focus = app.chrome.ui.keyboard.focused();
        assert!(!app.translate_workspace_chrome(&MSG {
            hwnd: app.documents[1].window.0,
            message: WM_KEYDOWN,
            wParam: WPARAM(VK_RETURN.0 as usize),
            ..Default::default()
        }));
        assert_eq!(app.chrome.ui.keyboard.focused(), focus);
        assert!(app.chrome.ui.pending.borrow().is_none());
        app.shutdown();
    }
}

#[test]
fn mouse_takeover_and_stale_owner_marker_cannot_keep_keyboard_source() {
    unsafe {
        let mut app = app();
        key(&app, VK_F6, &[VK_SHIFT], false);
        key(&app, VK_RETURN, &[VK_CONTROL], false);
        let port = at(0, Side::Output);
        let hwnd = strip(&app.chrome.ui, port).unwrap();
        let point = app.chrome.ui.point(port).unwrap();
        let mut client = POINT {
            x: point.x,
            y: point.y,
        };
        windows::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut client)
            .ok()
            .unwrap();
        input::mouse(
            hwnd,
            &app.chrome.ui,
            WM_LBUTTONDOWN,
            LPARAM(((client.y as u16 as u32) << 16 | client.x as u16 as u32) as isize),
        );
        assert!(app.chrome.ui.keyboard.source().is_none());
        assert!(app.chrome.ui.keyboard.focused().is_none());
        app.chrome.ui.cancel();
        let host = app.documents[0].window.0;
        let _ = SetFocus(Some(host));
        key(&app, VK_F6, &[], false);
        let marker = GetWindowLongPtrW(host, GWLP_USERDATA);
        SetWindowLongPtrW(host, GWLP_USERDATA, 0);
        reconcile(&app.chrome.ui);
        SetWindowLongPtrW(host, GWLP_USERDATA, marker);
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
