//! 真正的工具栏钉图设置与端点互斥；三角抽屉独立，消息仅投给测试自有 HWND。
use super::*;
use desktop::pin_settings as settings;

unsafe fn fixture() -> Application {
    let mut app = app();
    let doc = app
        .prepare_window(
            State::new(
                Some(crate::image_decode::Image {
                    width: 240,
                    height: 180,
                    pixels: vec![255; 240 * 180 * 4].into(),
                }),
                true,
            )
            .unwrap(),
            false,
        )
        .unwrap();
    app.documents[0] = doc;
    let doc = &app.documents[0];
    let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    desktop::edit_mode::toolbar_mode(doc.window.0, &doc.state, true).unwrap();
    let _ = ShowWindow(doc.panel.0, SW_SHOWNOACTIVATE);
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    app
}

unsafe fn settings_click(app: &Application) -> HWND {
    let doc = &app.documents[0];
    let button = GetDlgItem(Some(doc.panel.0), 0x6100).unwrap();
    assert!(IsWindowVisible(button).as_bool() && IsWindowEnabled(button).as_bool());
    SendMessageW(button, BM_CLICK, None, None);
    let mut message = MSG::default();
    while PeekMessageW(
        &mut message,
        Some(doc.window.0),
        desktop::tool_panel::EXECUTE,
        desktop::tool_panel::EXECUTE,
        PM_REMOVE,
    )
    .as_bool()
    {
        DispatchMessageW(&message);
    }
    let hwnd = settings::window(&doc.state).unwrap();
    assert!(settings::is_open(&doc.state) && IsWindowVisible(hwnd).as_bool());
    hwnd
}

fn port(side: Side) -> Port {
    Port {
        node: 0,
        side,
        index: 0,
    }
}
fn expanded(app: &Application, port: Port) -> bool {
    disclosure::expanded(
        &app.chrome.ui,
        port,
        &app.chrome.ui.node(port.node).unwrap(),
    )
}
unsafe fn tab(app: &mut Application) {
    let doc = &app.documents[0];
    SendMessageW(
        doc.window.0,
        WM_KEYDOWN,
        Some(WPARAM(VK_TAB.0 as usize)),
        None,
    );
    assert!(doc.state.parameters_ui.toggle.get());
    app.process_parameter_panels(true);
    app.sync_workspace_chrome(true);
}

#[test]
fn real_pin_settings_and_both_endpoint_sides_exclude_each_other_synchronously() {
    unsafe {
        let mut app = fixture();
        app.add_window(false).unwrap();
        let _ = ShowWindow(app.documents[1].window.0, SW_SHOWNOACTIVATE);
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let other = Port {
            node: 1,
            ..port(Side::Output)
        };
        presentation::endpoint_click(&app.chrome.ui, other);
        for side in [Side::Input, Side::Output] {
            let target = port(side);
            let settings = settings_click(&app);
            presentation::endpoint_click(&app.chrome.ui, target);
            assert!(expanded(&app, target));
            assert!(!settings::is_open(&app.documents[0].state));
            assert!(
                !IsWindowVisible(settings).as_bool(),
                "actual pin settings must hide before endpoint publication"
            );
            settings_click(&app);
            for side in [Side::Input, Side::Output] {
                assert!(
                    !expanded(&app, port(side)),
                    "settings must collapse local endpoint overrides immediately"
                );
            }
            app.sync_workspace_chrome(true);
            assert!(settings::is_open(&app.documents[0].state));
            assert!(expanded(&app, other));
            settings::close(&app.documents[0].state);
        }
        app.shutdown();
    }
}

#[test]
fn tab_and_settings_exchange_side_content_without_collapsing_the_bottom_drawer() {
    unsafe {
        let mut app = fixture();
        let open = select(&mut app);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        let settings = settings_click(&app);
        tab(&mut app);
        assert!(expanded(&app, port(Side::Output)));
        assert!(!IsWindowVisible(settings).as_bool());
        assert!(open.data.open.get());
        settings_click(&app);
        assert!(!expanded(&app, port(Side::Output)));
        assert!(!app.documents[0].state.parameters_ui.expanded.get());
        // Tab 可独立暴露抽屉，不依赖节点选择；设置切换不能撤销这个 Tab 可见意图。
        app.chrome.ui.selected.borrow_mut().clear();
        app.sync_workspace_chrome(true);
        assert!(
            open.data.open.get(),
            "side settings must not collapse the bottom drawer"
        );
        assert!(IsWindowVisible(open.window.0).as_bool());
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        click(&open, DRAWER_TOGGLE);
        app.sync_workspace_chrome(true);
        assert!(settings::is_open(&app.documents[0].state));
        assert!(open.data.open.get());
        app.shutdown();
    }
}

#[test]
fn parameter_focus_restores_existing_draft_and_closes_real_settings_only() {
    unsafe {
        let mut app = fixture();
        art(&mut app);
        let open = select(&mut app);
        tab(&mut app);
        let edit = text_edit(app.documents[0].state.parameters_ui.window(), "原值");
        SetWindowTextW(edit, w!("保留草稿")).unwrap();
        let settings = settings_click(&app);
        app.sync_workspace_chrome(true);
        assert!(!IsWindowVisible(edit).as_bool());
        assert!(open.data.open.get());
        let index = app
            .chrome
            .ui
            .scene
            .borrow()
            .as_ref()
            .unwrap()
            .catalog
            .endpoints[0]
            .inputs
            .len()
            - 1;
        assert!(parameter_rail::request(
            &app.chrome.ui,
            Port {
                index,
                ..port(Side::Input)
            },
            true
        ));
        assert!(!IsWindowVisible(settings).as_bool());
        assert!(open.data.open.get());
        app.sync_workspace_chrome(true);
        assert_eq!(
            text_edit(app.documents[0].state.parameters_ui.window(), "保留草稿"),
            edit
        );
        assert!(IsWindowVisible(edit).as_bool());
        assert_eq!(GetFocus(), edit);
        assert!(app.documents[0].state.pending_start.borrow().is_none());
        app.shutdown();
    }
}
