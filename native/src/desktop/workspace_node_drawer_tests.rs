//! 原 Application、自有 HWND 和真实 BUTTON 消息回归；不操作前台或用户窗口。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};
#[path = "workspace_pin_settings_disclosure_tests.rs"]
mod pin_settings;
#[path = "workspace_node_drawer_presentation_tests.rs"]
mod presentation;
unsafe fn app() -> Application {
    let mut app = super::super::tests::app();
    for doc in &app.documents {
        let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    }
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    app
}
unsafe fn select(app: &mut Application) -> Rc<Open> {
    selection::observe(
        &app.documents[0].state.chrome,
        app.documents[0].window.0,
        WM_LBUTTONDOWN,
    );
    app.sync_workspace_chrome(true);
    app.chrome.ui.drawers.borrow()[0].clone()
}
unsafe fn click(open: &Open, id: u16) {
    let button = GetDlgItem(Some(open.drawer.handle()), id as i32).unwrap();
    SendMessageW(button, BM_CLICK, None, None);
    let mut msg = MSG::default();
    while PeekMessageW(&mut msg, Some(open.window.0), NOTIFY, NOTIFY, PM_REMOVE).as_bool() {
        DispatchMessageW(&msg);
    }
}

#[test]
fn repeated_caret_clicks_publish_centered_drawer_and_leave_no_black_caret_box() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        let root = open.drawer.handle();
        for _ in 0..12 {
            for expanded in [true, false] {
                click(&open, DRAWER_TOGGLE);
                app.sync_workspace_chrome(true);
                assert_eq!(open.drawer.handle(), root);
                let mut pin = RECT::default();
                let mut drawer = RECT::default();
                // 抽屉附着实际图片客户区，不包含普通窗口的不可见 resize frame。
                let host = app.documents[0].window.0;
                GetClientRect(host, &mut pin).unwrap();
                let mut origin = POINT::default();
                ClientToScreen(host, &mut origin).ok().unwrap();
                OffsetRect(&mut pin, origin.x, origin.y).ok().unwrap();
                GetWindowRect(open.window.0, &mut drawer).unwrap();
                assert_eq!(drawer.top, pin.bottom);
                assert!((drawer.left + drawer.right - pin.left - pin.right).abs() <= 1);
                let region = CreateRectRgn(0, 0, 0, 0);
                assert_ne!(GetWindowRgn(open.window.0, region), RGN_ERROR);
                assert_eq!(PtInRegion(region, 0, 0).as_bool(), expanded);
                DeleteObject(HGDIOBJ(region.0)).unwrap();
                assert!(!GetUpdateRect(root, None, false).as_bool());
            }
        }
        app.shutdown();
    }
}
#[test]
fn collapsing_with_hidden_endpoints_does_not_restore_the_host_visible_bit() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        let mut node = app
            .chrome
            .ui
            .nodes
            .borrow()
            .iter()
            .find(|n| open.matches(n))
            .unwrap()
            .clone();
        let model = open.data.model.borrow().clone();
        open.data.set_open(true).unwrap();
        node.hide_collapsed_endpoints = true;
        open.update(&node, model.clone()).unwrap();
        assert!(IsWindowVisible(open.window.0).as_bool());
        open.data.set_open(false).unwrap();
        open.update(&node, model).unwrap();
        assert!(!IsWindowVisible(open.window.0).as_bool());
        app.shutdown();
    }
}
unsafe fn art(app: &mut Application) {
    let (_listener, identity, _) = crate::loom_bridge_client::tests::fixture();
    let mut value = crate::loom_handshake::tests::fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"] =
        serde_json::json!([{"id":"text","type":"string","default":"原值"}]);
    let handshake = crate::loom_handshake::response(&serde_json::to_vec(&value).unwrap())
        .unwrap()
        .unwrap();
    *app.documents[0].state.selected_art.borrow_mut() =
        Some(desktop::capability_selection::Selection {
            execution: crate::graph_execution_config::Config {
                param_driven: false,
                ..Default::default()
            },
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
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
}
unsafe fn text_edit(panel: HWND, expected: &str) -> HWND {
    struct Search<'a> {
        hwnd: HWND,
        expected: &'a str,
    }
    unsafe extern "system" fn find(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
        let result = &mut *(lp.0 as *mut Search<'_>);
        let mut class = [0; 32];
        let mut value = [0; 32];
        let n = GetClassNameW(hwnd, &mut class) as usize;
        let m = GetWindowTextW(hwnd, &mut value) as usize;
        if String::from_utf16_lossy(&class[..n]).eq_ignore_ascii_case("EDIT")
            && String::from_utf16_lossy(&value[..m]) == result.expected
        {
            result.hwnd = hwnd;
        }
        windows::core::BOOL::from(result.hwnd.is_invalid())
    }
    let mut result = Search {
        hwnd: HWND::default(),
        expected,
    };
    let _ = EnumChildWindows(
        Some(panel),
        Some(find),
        LPARAM(&mut result as *mut Search<'_> as isize),
    );
    assert!(!result.hwnd.is_invalid());
    result.hwnd
}
#[test]
fn art_bottom_buttons_commit_existing_parameter_drafts_before_setting_or_run() {
    unsafe {
        let mut app = app();
        art(&mut app);
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        let open = app.chrome.ui.drawers.borrow()[0].clone();
        assert!(open.data.open.get());
        assert!(projection::buttons(&open.data.model.borrow())
            .iter()
            .all(|b| b.icon.validate().is_ok()));
        assert!(GetDlgItem(Some(open.drawer.handle()), 6).is_err());
        let edit = text_edit(app.documents[0].state.parameters_ui.window(), "原值");
        SetWindowTextW(edit, w!("新草稿")).unwrap();
        let before = app.documents[0]
            .state
            .selected_art
            .borrow()
            .as_ref()
            .unwrap()
            .execution
            .accepts_upstream_notification();
        click(&open, 3);
        app.sync_workspace_chrome(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        {
            let selected = app.documents[0].state.selected_art.borrow();
            let selected = selected.as_ref().unwrap();
            assert_eq!(
                selected.parameters.get("text").flatten(),
                Some(&serde_json::json!("新草稿"))
            );
            assert_eq!(
                (
                    selected.execution.upstream_driven,
                    selected.execution.listen_upstream
                ),
                (!before, !before)
            );
        }
        // 草稿提交会排入原参数触发；设置换代使它过期，由原调度入口退休，不启动网络。
        app.start_requested_preparations(false);
        app.sync_workspace_chrome(true);
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        let edit = text_edit(app.documents[0].state.parameters_ui.window(), "新草稿");
        SetWindowTextW(edit, w!("执行草稿")).unwrap();
        click(&open, RUN);
        app.sync_workspace_chrome(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        assert_eq!(
            app.documents[0]
                .state
                .selected_art
                .borrow()
                .as_ref()
                .unwrap()
                .parameters
                .get("text")
                .flatten(),
            Some(&serde_json::json!("执行草稿"))
        );
        assert!(app.documents[0]
            .state
            .pending_start
            .borrow()
            .as_ref()
            .unwrap()
            .stamp
            .matches(&app.documents[0].state));
        assert!(
            !IsWindowEnabled(GetDlgItem(Some(open.drawer.handle()), RUN as i32).unwrap()).as_bool()
        );
        app.shutdown();
    }
}
#[test]
fn selection_is_explicit_not_mode_or_focus_and_drawer_toggles_real_settings() {
    unsafe {
        let mut app = app();
        assert!(app.chrome.ui.drawers.borrow().is_empty());
        let _ = SetFocus(Some(app.documents[0].window.0));
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.drawers.borrow().is_empty());
        let open = select(&mut app);
        assert!(!open.data.open.get());
        assert!(IsWindowVisible(open.window.0).as_bool());
        assert!(GetWindow(open.window.0, GW_OWNER)
            .unwrap_or_default()
            .is_invalid());
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        assert!(open.data.open.get());
        let root = open.drawer.handle();
        let notify = GetDlgItem(Some(root), 5).unwrap();
        assert!(IsWindowVisible(notify).as_bool());
        click(&open, 5);
        app.sync_workspace_chrome(true);
        assert!(!app.documents[0].state.node_settings.get().notify_downstream);
        assert!(app.documents[0].state.accept_upstream.get());
        assert_eq!(GetDlgItem(Some(root), 5).unwrap(), notify);
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        drop(open);
        let weak = app.chrome.bind();
        app.shutdown();
        drop(app);
        assert!(weak.upgrade().is_none());
    }
}
#[test]
fn drawer_focus_keeps_the_existing_editor_toggle_on_its_document() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let state = &app.documents[0].state;
        let toolbar = state.tool_panel.get();
        for visible in [true, false] {
            let button = GetDlgItem(Some(open.drawer.handle()), 5).unwrap();
            let _ = SetFocus(Some(button));
            assert!(translate(
                &app.chrome.ui,
                &MSG {
                    hwnd: button,
                    message: WM_KEYDOWN,
                    wParam: WPARAM(VK_F4.0 as usize),
                    ..Default::default()
                }
            ));
            assert_eq!(IsWindowVisible(toolbar).as_bool(), visible);
        }
        assert_eq!(state.canvas.borrow().tool, crate::editor::Tool::Pan);
        app.shutdown();
    }
}
#[test]
fn tab_edges_resync_independent_drawer_while_hidden_locked_states_reject_clicks() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        assert!(open.data.open.get());
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        assert!(!open.data.open.get());
        assert!(app.documents[0].state.parameters_ui.expanded.get());
        app.sync_workspace_chrome(true);
        assert!(!open.data.open.get());
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        assert!(!open.data.open.get());
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        click(&open, 5);
        app.documents[0].state.group_locked.set(true);
        app.sync_workspace_chrome(true);
        assert!(app.documents[0].state.node_settings.get().notify_downstream);
        app.documents[0].state.group_locked.set(false);
        app.sync_workspace_chrome(true);
        click(&open, 5);
        app.sync_workspace_chrome(false);
        app.sync_workspace_chrome(true);
        assert!(app.documents[0].state.node_settings.get().notify_downstream);
        app.documents[0].state.group_hidden.set(true);
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.drawers.borrow().is_empty());
        app.shutdown();
    }
}
#[test]
fn old_context_and_replaced_source_are_not_reinterpreted_as_current_clicks() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let old = open.data.context.get();
        app.documents[0].state.accept_upstream.set(false);
        app.sync_workspace_chrome(true);
        SendMessageW(
            open.window.0,
            NOTIFY,
            Some(WPARAM(5 | ((old as usize) << 32))),
            Some(LPARAM(open.drawer.handle().0 as isize)),
        );
        app.sync_workspace_chrome(true);
        assert!(app.documents[0].state.node_settings.get().notify_downstream);
        assert!(open.data.pending.get().is_none());
        let host = app.documents[0].window.0;
        DestroyWindow(host).unwrap();
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.drawers.borrow().is_empty());
        assert!(app.chrome.ui.selected.borrow().is_empty());
        app.shutdown();
    }
}
#[test]
fn collapse_retires_queued_clicks_and_closing_drawer_does_not_close_global_chrome() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let old = open.data.context.get();
        assert!(translate(
            &app.chrome.ui,
            &MSG {
                hwnd: GetDlgItem(Some(open.drawer.handle()), 5).unwrap(),
                message: WM_KEYDOWN,
                wParam: WPARAM(VK_ESCAPE.0 as usize),
                ..Default::default()
            }
        ));
        SendMessageW(
            open.window.0,
            NOTIFY,
            Some(WPARAM(5 | ((old as usize) << 32))),
            Some(LPARAM(open.drawer.handle().0 as isize)),
        );
        app.sync_workspace_chrome(true);
        assert!(app.documents[0].state.node_settings.get().notify_downstream);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        SendMessageW(open.window.0, WM_CLOSE, None, None);
        app.sync_workspace_chrome(true);
        assert!(open.drawer.alive());
        assert!(!open.data.open.get());
        assert!(app.chrome.ui.failure.borrow().is_none());
        DestroyWindow(open.window.0).unwrap();
        app.sync_workspace_chrome(true);
        assert!(!open.drawer.alive());
        assert!(app.chrome.ui.drawers.borrow()[0].drawer.alive());
        assert!(app.chrome.ui.failure.borrow().is_none());
        app.shutdown();
    }
}
#[test]
fn attached_geometry_moves_in_cached_frame_batch_and_stays_below_pins() {
    unsafe {
        let mut app = app();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let host = app.documents[0].window.0;
        let frame = app.chrome.ui.borders.borrow()[0].hwnd;
        let mut before = RECT::default();
        GetWindowRect(open.window.0, &mut before).unwrap();
        let mut outer = RECT::default();
        GetWindowRect(frame, &mut outer).unwrap();
        movement::cached(
            Some(&app.chrome.ui),
            host,
            frame,
            outer.left + 31,
            outer.top + 27,
            &[],
        )
        .unwrap();
        let mut after = RECT::default();
        GetWindowRect(open.window.0, &mut after).unwrap();
        assert_eq!((after.left - before.left, after.top - before.top), (31, 27));
        let mut cursor = GetTopWindow(None).unwrap();
        let mut pin = false;
        for _ in 0..4096 {
            if cursor == host {
                pin = true;
            }
            if cursor == open.window.0 {
                break;
            }
            cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
        }
        assert_eq!(cursor, open.window.0);
        assert!(pin);
        let mut affinity = 0;
        GetWindowDisplayAffinity(open.window.0, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        assert!(app.chrome.handles().contains(&open.window.0));
        app.shutdown();
    }
}
