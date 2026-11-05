//! 实际端点条child、唯一事务和退休围栏；仅自有HWND，不注册热键或合成全局输入。
use super::*;
#[path = "workspace_chrome_capsule_reentry_tests.rs"]
mod reentry;

unsafe fn app() -> Application {
    let mut app = super::super::tests::app();
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"] = serde_json::json!([
        {"id":"text","type":"string","default":"原值"},
        {"id":"n","widget":"number","default":3,"min":0,"max":10,"step":0.5},
        {"id":"enabled","type":"boolean","default":false},
        {"id":"secret","secret":true,"default":"DO_NOT_RENDER"}
    ]);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    *app.documents[0].state.selected_art.borrow_mut() =
        Some(desktop::capability_selection::Selection {
            execution: Default::default(),
            identity: desktop::document_state::new_unit_id().unwrap(),
            parameters: crate::loom_parameter_state::Parameters::new(
                &handshake.capability(0).unwrap(),
            )
            .unwrap(),
            catalog: Rc::new(desktop::capability_job::Loaded {
                identity,
                handshake,
            }),
        });
    let host = app.documents[0].window.0;
    SetWindowPos(
        host,
        None,
        420,
        160,
        400,
        300,
        SWP_NOACTIVATE | SWP_NOZORDER,
    )
    .unwrap();
    let _ = ShowWindow(host, SW_SHOWNOACTIVATE);
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    let _ = SetFocus(Some(host));
    assert_eq!(GetFocus(), host);
    app
}
fn port(app: &Application, field: &str) -> Port {
    let scene = app.chrome.ui.scene.borrow().clone().unwrap();
    Port {
        node: 0,
        side: Side::Input,
        index: scene.catalog.endpoints[0]
            .inputs
            .iter()
            .position(|p| p == field)
            .unwrap(),
    }
}
unsafe fn open(app: &mut Application, field: &str, enter: bool) -> Rc<Panel> {
    request(&app.chrome.ui, port(app, field), enter);
    app.sync_workspace_chrome(true);
    assert!(app.chrome.ui.failure.borrow().is_none());
    selected(&app.chrome.ui)
        .unwrap()
        .panel
        .borrow()
        .clone()
        .unwrap()
}
unsafe fn edit(panel: &Panel) -> HWND {
    FindWindowExW(Some(panel.handle()), None, w!("EDIT"), None).unwrap()
}
unsafe fn text(hwnd: HWND) -> String {
    let mut out = vec![0; GetWindowTextLengthW(hwnd).max(0) as usize + 1];
    let length = GetWindowTextW(hwnd, &mut out).max(0) as usize;
    String::from_utf16(&out[..length]).unwrap()
}
unsafe fn commit(panel: &Panel, value: &str) {
    let edit = edit(panel);
    SetWindowTextW(edit, &HSTRING::from(value)).unwrap();
    SendMessageW(
        panel.handle(),
        WM_COMMAND,
        Some(WPARAM((EN_KILLFOCUS as usize) << 16)),
        Some(LPARAM(edit.0 as isize)),
    );
}
fn value(app: &Application, field: &str) -> serde_json::Value {
    app.documents[0]
        .state
        .selected_art
        .borrow()
        .as_ref()
        .unwrap()
        .parameters
        .get(field)
        .flatten()
        .unwrap()
        .clone()
}
unsafe fn key(app: &Application, key: u16, repeat: bool) -> bool {
    app.translate_workspace_chrome(&MSG {
        hwnd: GetFocus(),
        message: WM_KEYDOWN,
        wParam: WPARAM(key as usize),
        lParam: LPARAM(if repeat { 1 << 30 } else { 0 }),
        ..Default::default()
    })
}
#[test]
fn actual_capsule_is_inside_the_owned_strip_and_region_without_covering_image_pixels() {
    unsafe {
        let mut app = app();
        let panel = open(&mut app, "text", true);
        let open = selected(&app.chrome.ui).unwrap();
        assert_eq!(GetParent(panel.handle()).unwrap(), open.parent);
        assert!(IsChild(open.parent, edit(&panel)).as_bool());
        assert!(app.documents[0].state.parameters_ui.window().is_invalid());
        let mut rect = RECT::default();
        GetWindowRect(panel.handle(), &mut rect).unwrap();
        let node = app.chrome.ui.node(0).unwrap();
        assert!(rect.right <= node.rect.left || rect.left >= node.rect.right);
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(open.parent, region), RGN_ERROR);
        let mut point = POINT {
            x: (rect.left + rect.right) / 2,
            y: (rect.top + rect.bottom) / 2,
        };
        ScreenToClient(open.parent, &mut point).ok().unwrap();
        assert!(PtInRegion(region, point.x, point.y).as_bool());
        DeleteObject(HGDIOBJ(region.0)).ok().unwrap();
        assert_eq!(GetFocus(), edit(&panel));
        assert!(app.chrome.idle());
        app.shutdown();
        assert!(panel.retired());
    }
}
#[test]
fn dirty_text_blocks_idle_and_only_its_successful_commit_may_mount_a_fresh_successor() {
    unsafe {
        let mut app = app();
        let panel = open(&mut app, "text", true);
        SetWindowTextW(edit(&panel), &HSTRING::from("中文🙂")).unwrap();
        assert!(!app.chrome.idle());
        let original = selected(&app.chrome.ui).unwrap();
        commit(&panel, "中文🙂");
        app.sync_workspace_chrome(true);
        assert_eq!(value(&app, "text"), "中文🙂");
        let successor = selected(&app.chrome.ui).expect("own commit keeps an active fresh capsule");
        assert!(!Rc::ptr_eq(&successor, &original));
        assert!(!Rc::ptr_eq(&successor.catalog, &original.catalog));
        assert!(panel.retired());
        let next = successor.panel.borrow().clone().unwrap();
        assert_eq!(text(edit(&next)), "中文🙂");
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn f2_enters_real_edit_while_f6_and_two_stage_escape_return_to_ports_without_host_deletion() {
    unsafe {
        let mut app = app();
        keyboard::restore(&app.chrome.ui, port(&app, "text"));
        assert!(key(&app, VK_F2.0, false));
        app.sync_workspace_chrome(true);
        let panel = selected(&app.chrome.ui)
            .unwrap()
            .panel
            .borrow()
            .clone()
            .unwrap();
        let edit = edit(&panel);
        assert_eq!(GetFocus(), edit);
        SetWindowTextW(edit, &HSTRING::from("放弃草稿")).unwrap();
        assert!(key(&app, VK_ESCAPE.0, false));
        assert_eq!(text(edit), "原值");
        assert!(!panel.retired());
        assert!(key(&app, VK_ESCAPE.0, false));
        assert!(panel.retired());
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(port(&app, "text")));
        assert!(key(&app, VK_ESCAPE.0, true));
        assert!(app.documents[0].is_alive());
        app.sync_workspace_chrome(true);
        let panel = open(&mut app, "n", true);
        assert!(key(&app, VK_F6.0, false));
        assert!(panel.retired());
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(port(&app, "n")));
        assert!(key(&app, VK_F6.0, true));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(port(&app, "n")));
        app.shutdown();
    }
}
#[test]
fn one_slot_replacement_and_output_disclosure_never_create_per_port_editors() {
    unsafe {
        let mut app = app();
        let first = open(&mut app, "text", false);
        let second = open(&mut app, "n", false);
        assert!(first.retired());
        assert!(!second.retired());
        let scene = app.chrome.ui.scene.borrow().clone().unwrap();
        assert!(!scene.catalog.endpoints[0]
            .inputs
            .iter()
            .any(|p| p == "secret"));
        cancel(&app.chrome.ui);
        request(
            &app.chrome.ui,
            Port {
                node: 0,
                side: Side::Output,
                index: 0,
            },
            true,
        );
        app.sync_workspace_chrome(true);
        assert!(selected(&app.chrome.ui).is_none());
        assert!(second.retired());
        app.shutdown();
    }
}
#[test]
fn stale_config_hidden_locked_capture_and_port_visibility_reject_queued_drafts() {
    unsafe {
        for variant in 0..5 {
            let mut app = app();
            let panel = open(&mut app, "text", true);
            commit(&panel, "不能提交");
            let state = &app.documents[0].state;
            match variant {
                0 => {
                    state
                        .selected_art
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .parameters
                        .set("n", Some(serde_json::json!(7)))
                        .unwrap();
                }
                1 => state.group_hidden.set(true),
                2 => state.group_locked.set(true),
                3 => app.capture_active.set(true),
                _ => {
                    app.workspace
                        .set_port_visibility(
                            &state.unit_id,
                            crate::workspace::ports::Visibility::read(r#"{"text":false}"#).unwrap(),
                        )
                        .unwrap();
                }
            }
            app.sync_workspace_chrome(true);
            assert_eq!(value(&app, "text"), "原值", "variant={variant}");
            assert!(panel.retired(), "variant={variant}");
            assert!(selected(&app.chrome.ui).is_none());
            app.capture_active.set(false);
            app.shutdown();
        }
    }
}
#[test]
fn external_focus_and_mouse_capture_retire_drafts_without_a_late_blur_commit() {
    unsafe {
        let mut app = app();
        app.add_window(false).unwrap();
        let _ = ShowWindow(app.documents[1].window.0, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let panel = open(&mut app, "text", true);
        SetWindowTextW(edit(&panel), &HSTRING::from("失焦不提交")).unwrap();
        let _ = SetFocus(Some(app.documents[1].window.0));
        app.sync_workspace_chrome(true);
        assert_eq!(value(&app, "text"), "原值");
        assert!(panel.retired());
        let _ = SetFocus(Some(app.documents[0].window.0));
        let panel = open(&mut app, "text", true);
        app.chrome.ui.cancel();
        window::hide(&app.chrome.ui);
        assert!(panel.retired());
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn synchronous_child_focus_cancellation_cannot_restore_a_retired_candidate() {
    unsafe extern "system" fn retire(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
        _: usize,
        data: usize,
    ) -> LRESULT {
        if msg == WM_SETFOCUS {
            cancel(&*(data as *const Ui));
        }
        windows::Win32::UI::Shell::DefSubclassProc(hwnd, msg, wp, lp)
    }
    unsafe {
        let mut app = app();
        let panel = open(&mut app, "text", false);
        let edit = edit(&panel);
        windows::Win32::UI::Shell::SetWindowSubclass(
            edit,
            Some(retire),
            536,
            Rc::as_ptr(&app.chrome.ui) as usize,
        )
        .ok()
        .unwrap();
        request(&app.chrome.ui, port(&app, "text"), true);
        app.sync_workspace_chrome(true);
        assert!(selected(&app.chrome.ui).is_none());
        assert!(panel.retired());
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
