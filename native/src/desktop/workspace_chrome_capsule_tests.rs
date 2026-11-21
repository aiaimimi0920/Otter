//! 局部展开/F2 与 Tab 共用真实行内 owner；不注册热键或合成全局输入。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};
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
    tick(&mut app);
    let _ = SetFocus(Some(host));
    assert_eq!(GetFocus(), host);
    app
}
unsafe fn tick(app: &mut Application) {
    app.process_parameter_panels(true);
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    assert!(app.chrome.ui.failure.borrow().is_none());
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
fn panel(app: &Application) -> &Panel {
    app.documents[0].parameters.as_ref().unwrap()
}
unsafe fn open(app: &mut Application, field: &str, enter: bool) -> HWND {
    assert!(request(&app.chrome.ui, port(app, field), enter));
    tick(app);
    panel(app).handle()
}
unsafe fn edit(app: &Application, field: &str) -> HWND {
    GetDlgItem(
        Some(panel(app).handle()),
        if field == "text" { 1000 } else { 1001 },
    )
    .unwrap()
}
unsafe fn text(hwnd: HWND) -> String {
    let mut out = vec![0; GetWindowTextLengthW(hwnd).max(0) as usize + 1];
    let length = GetWindowTextW(hwnd, &mut out).max(0) as usize;
    String::from_utf16(&out[..length]).unwrap()
}
unsafe fn commit(app: &Application, value: &str) {
    let edit = edit(app, "text");
    SetWindowTextW(edit, &HSTRING::from(value)).unwrap();
    SendMessageW(
        panel(app).handle(),
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
    let msg = MSG {
        hwnd: GetFocus(),
        message: WM_KEYDOWN,
        wParam: WPARAM(key as usize),
        lParam: LPARAM(if repeat { 1 << 30 } else { 0 }),
        ..Default::default()
    };
    // 仿真实消息泵：chrome 未消费时再交文档参数控件，不重复生产分派。
    app.translate_workspace_chrome(&msg)
        || app
            .documents
            .iter()
            .any(|d| d.parameters.as_ref().is_some_and(|p| p.translate(&msg)))
}
#[test]
fn local_row_is_inside_the_strip_with_only_real_controls_in_its_hit_region() {
    unsafe {
        let mut app = app();
        let hwnd = open(&mut app, "text", true);
        let parent = GetParent(hwnd).unwrap();
        assert!(app
            .chrome
            .ui
            .input_windows
            .borrow()
            .iter()
            .any(|(h, n, s)| *h == parent && *n == 0 && *s == Side::Input));
        let child = edit(&app, "text");
        assert!(IsChild(parent, child).as_bool());
        assert_eq!(app.documents[0].state.parameters_ui.window(), hwnd);
        let mut rect = RECT::default();
        GetWindowRect(child, &mut rect).unwrap();
        assert!(rect.right <= app.chrome.ui.node(0).unwrap().rect.left);
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(hwnd, region), RGN_ERROR);
        let mut point = POINT {
            x: (rect.left + rect.right) / 2,
            y: (rect.top + rect.bottom) / 2,
        };
        ScreenToClient(hwnd, &mut point).ok().unwrap();
        assert!(PtInRegion(region, point.x, point.y).as_bool());
        assert_ne!(GetWindowRgn(parent, region), RGN_ERROR);
        point = POINT {
            x: (rect.left + rect.right) / 2,
            y: (rect.top + rect.bottom) / 2,
        };
        ScreenToClient(parent, &mut point).ok().unwrap();
        assert!(
            PtInRegion(region, point.x, point.y).as_bool(),
            "the actual parent must not clip the local field"
        );
        DeleteObject(HGDIOBJ(region.0)).unwrap();
        assert_eq!(GetFocus(), child);
        assert!(app.chrome.idle());
        app.shutdown();
        assert!(!IsWindow(Some(hwnd)).as_bool());
    }
}
#[test]
fn dirty_local_row_commits_through_the_document_owner_and_preserves_disclosure() {
    unsafe {
        let mut app = app();
        let hwnd = open(&mut app, "text", false);
        SetWindowTextW(edit(&app, "text"), &HSTRING::from("中文🙂")).unwrap();
        assert!(!app.chrome.idle());
        commit(&app, "中文🙂");
        tick(&mut app);
        assert_eq!(value(&app, "text"), "中文🙂");
        assert_eq!(panel(&app).handle(), hwnd);
        assert!(disclosure::expanded(
            &app.chrome.ui,
            port(&app, "text"),
            &app.chrome.ui.node(0).unwrap()
        ));
        assert_eq!(text(edit(&app, "text")), "中文🙂");
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn f2_f6_and_two_stage_escape_keep_the_pin_and_restore_the_corresponding_port() {
    unsafe {
        let mut app = app();
        keyboard::restore(&app.chrome.ui, port(&app, "text"));
        assert!(key(&app, VK_F2.0, false));
        tick(&mut app);
        let child = edit(&app, "text");
        assert_eq!(GetFocus(), child);
        SetWindowTextW(child, &HSTRING::from("放弃草稿")).unwrap();
        assert!(key(&app, VK_ESCAPE.0, false));
        assert_eq!(text(child), "原值");
        assert!(!panel(&app).retired());
        assert!(key(&app, VK_ESCAPE.0, false));
        assert!(panel(&app).retired());
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(port(&app, "text")));
        assert!(key(&app, VK_ESCAPE.0, true));
        tick(&mut app);
        let hwnd = open(&mut app, "n", true);
        assert_eq!(GetFocus(), edit(&app, "n"));
        assert!(key(&app, VK_F6.0, false));
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(port(&app, "n")));
        assert!(
            !panel(&app).retired(),
            "F6 leaves focus without closing the inline form"
        );
        assert_eq!(panel(&app).handle(), hwnd);
        assert!(key(&app, VK_F6.0, true));
        assert!(app.documents[0].is_alive());
        app.shutdown();
    }
}
#[test]
fn independent_local_rows_and_tab_share_one_owner_without_focus_theft() {
    unsafe {
        let mut app = app();
        let host = app.documents[0].window.0;
        let hwnd = open(&mut app, "text", false);
        let child = edit(&app, "text");
        assert_eq!(GetFocus(), host);
        assert_eq!(open(&mut app, "n", false), hwnd);
        assert_eq!(GetFocus(), host);
        assert!(IsWindowVisible(child).as_bool());
        assert!(IsWindowVisible(edit(&app, "n")).as_bool());
        assert!(!request(
            &app.chrome.ui,
            Port {
                node: 0,
                side: Side::Output,
                index: 0
            },
            true
        ));
        assert!(!app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .unwrap()
            .catalog
            .endpoints[0]
            .inputs
            .iter()
            .any(|p| p == "secret"));
        app.documents[0].state.parameters_ui.toggle.set(true);
        tick(&mut app);
        assert_eq!(panel(&app).handle(), hwnd);
        assert_eq!(edit(&app, "text"), child);
        assert!(app.documents[0].state.parameters_ui.expanded.get());
        app.shutdown();
    }
}
#[test]
fn folding_all_local_rows_keeps_dirty_drafts_and_the_idle_gate_until_reopened() {
    unsafe {
        let mut app = app();
        let hwnd = open(&mut app, "text", false);
        let child = edit(&app, "text");
        SetWindowTextW(child, &HSTRING::from("保留草稿")).unwrap();
        assert!(!disclosure::toggle(&app.chrome.ui, port(&app, "text")));
        tick(&mut app);
        assert_eq!(panel(&app).handle(), hwnd);
        assert!(!IsWindowVisible(hwnd).as_bool());
        assert!(!app.chrome.idle());
        assert!(!panel(&app).exit_idle());
        assert_eq!(open(&mut app, "text", false), hwnd);
        assert_eq!(edit(&app, "text"), child);
        assert_eq!(text(child), "保留草稿");
        assert!(IsWindowVisible(hwnd).as_bool());
        commit(&app, "保留草稿");
        tick(&mut app);
        assert_eq!(value(&app, "text"), "保留草稿");
        app.shutdown();
    }
}
#[test]
fn stale_config_hidden_locked_capture_and_modal_changes_reject_queued_drafts() {
    unsafe {
        for variant in 0..5 {
            let mut app = app();
            open(&mut app, "text", false);
            commit(&app, "不能提交");
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
                3 => state.capture_active.set(true),
                _ => state.file_dialog.set(true),
            }
            tick(&mut app);
            assert_eq!(value(&app, "text"), "原值", "variant={variant}");
            app.documents[0].state.capture_active.set(false);
            app.documents[0].state.file_dialog.set(false);
            app.shutdown();
        }
    }
}
#[test]
fn local_edit_blur_uses_the_same_valid_commit_as_the_full_tab_form() {
    unsafe {
        let mut app = app();
        open(&mut app, "text", true);
        SetWindowTextW(edit(&app, "text"), &HSTRING::from("失焦提交")).unwrap();
        let _ = SetFocus(Some(app.documents[0].window.0));
        tick(&mut app);
        assert_eq!(value(&app, "text"), "失焦提交");
        app.shutdown();
    }
}
