//! 节点级隐藏同时约束实际条、绘制与键盘；不删端口、不改变连线/图片或其它钉图。
use super::*;

#[test]
fn collapsed_endpoint_setting_hides_real_strips_and_tab_reveals_them_again() {
    unsafe {
        let mut app = visible_app();
        let handles = app.chrome.handles();
        let unit = app.documents[0].state.unit_id.clone();
        let save = desktop::application::save_state::Stamp::capture(&app).unwrap();
        let topology = app.workspace.topology_stamp();
        let config = desktop::configuration_stamp::Stamp::capture(&app.documents[0].state);
        let _ = SetFocus(Some(app.documents[0].window.0));
        assert!(keyboard::translate(
            &app.chrome.ui,
            &MSG {
                hwnd: app.documents[0].window.0,
                message: WM_KEYDOWN,
                wParam: WPARAM(VK_F6.0 as usize),
                ..Default::default()
            }
        ));
        assert!(app.chrome.ui.keyboard.focused().is_some());
        desktop::node_settings::apply(
            app.documents[0].window.0,
            &app.documents[0].state,
            desktop::node_settings::Action::Endpoints,
        )
        .unwrap();
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.node(0).unwrap().hide_collapsed_endpoints);
        assert!(app.chrome.ui.keyboard.focused().is_none());
        assert!(draw::ports(&app.chrome.ui)
            .iter()
            .all(|(port, _, _)| port.node != 0));
        for (hwnd, node, _) in app.chrome.ui.input_windows.borrow().iter() {
            assert_eq!(IsWindowVisible(*hwnd).as_bool(), *node != 0);
        }
        assert!(topology.matches(&app.workspace));
        assert!(config.matches(&app.documents[0].state));
        assert!(!save.matches_model(&app));
        assert_eq!(app.documents[0].state.unit_id, unit);
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.node(0).unwrap().expanded);
        assert!(draw::ports(&app.chrome.ui)
            .iter()
            .any(|(port, _, _)| port.node == 0));
        for (hwnd, _, _) in app.chrome.ui.input_windows.borrow().iter() {
            assert!(IsWindowVisible(*hwnd).as_bool());
        }
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        assert!(draw::ports(&app.chrome.ui)
            .iter()
            .all(|(port, _, _)| port.node != 0));
        assert_eq!(app.chrome.handles(), handles);
        app.shutdown();
    }
}
#[test]
fn each_node_setting_invalidates_save_receipt_without_dirtying_editor() {
    unsafe {
        let mut app = visible_app();
        for action in [
            desktop::node_settings::Action::Endpoints,
            desktop::node_settings::Action::Notify,
        ] {
            let save = desktop::application::save_state::Stamp::capture(&app).unwrap();
            let source =
                desktop::application::restore_batch::guard::Document::capture(&app.documents[0])
                    .unwrap();
            let editor = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .document_stamp();
            desktop::node_settings::apply(
                app.documents[0].window.0,
                &app.documents[0].state,
                action,
            )
            .unwrap();
            assert!(!save.matches_model(&app));
            assert!(!source.matches(&app.documents[0]));
            assert!(editor.matches(&app.documents[0].state.canvas.borrow().editor));
        }
        app.shutdown();
    }
}
