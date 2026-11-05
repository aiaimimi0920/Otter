//! 自有Application的外观事务；可见窗仅在runner的隔离desktop中使用。
use super::*;
use crate::workspace::ports::Visibility;
unsafe fn visible() -> Application {
    let mut app = super::super::tests::app();
    for doc in &app.documents {
        SetWindowPos(
            doc.window.0,
            None,
            240,
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
    app
}
unsafe fn queued(app: &Application) -> Draft {
    let doc = &app.documents[0];
    assert!(request(&doc.state.chrome, doc.window.0));
    assert!(!app.chrome.idle());
    assert!(!request(&doc.state.chrome, doc.window.0));
    let draft = app.prepare_port_settings().unwrap();
    assert!(app.chrome.idle());
    draft
}
#[test]
fn document_menu_routes_single_slot_and_close_releases_open() {
    unsafe {
        let mut app = visible();
        let doc = &app.documents[0];
        let menu = CreatePopupMenu().unwrap();
        let snapshot = desktop::node_menu::Snapshot::append(menu, &doc.state).unwrap();
        assert_ne!(GetMenuState(menu, COMMAND as u32, MF_BYCOMMAND), u32::MAX);
        snapshot.apply(doc.window.0, &doc.state, COMMAND).unwrap();
        DestroyMenu(menu).unwrap();
        assert!(!app.chrome.idle());
        app.chrome.close();
        assert!(app.chrome.idle());
        app.shutdown();
    }
}
#[test]
fn commit_changes_only_appearance_and_rejects_stale_owner_catalog_or_global_gate() {
    unsafe {
        for invalid in 0..6 {
            let mut app = visible();
            let draft = queued(&app);
            let unit = draft.request.unit.clone();
            let topology = app.workspace.topology_stamp();
            let config = desktop::configuration_stamp::Stamp::capture(&app.documents[0].state);
            let value = Visibility::read(r#"{"output":false}"#).unwrap();
            match invalid {
                1 => app.documents[0].state.group_locked.set(true),
                2 => {
                    let _ = ShowWindow(draft.request.hwnd, SW_HIDE);
                }
                3 => {
                    app.workspace
                        .set_port_visibility(&unit, Visibility::read(r#"{"image":false}"#).unwrap())
                        .unwrap();
                }
                4 => app.capture_active.set(true),
                5 => {
                    SetWindowLongPtrW(draft.request.hwnd, GWLP_USERDATA, draft.request.marker + 1);
                }
                _ => (),
            }
            app.commit_port_settings(&draft.request, &draft.catalog, value);
            assert_eq!(
                app.workspace.port_visibility(&unit).get("output"),
                if invalid == 0 { Some(false) } else { None }
            );
            assert!(topology.matches(&app.workspace));
            assert!(config.matches(&app.documents[0].state));
            if invalid == 5 {
                SetWindowLongPtrW(draft.request.hwnd, GWLP_USERDATA, draft.request.marker);
            }
            app.capture_active.set(false);
            app.shutdown();
        }
    }
}
#[test]
fn catalog_masks_do_not_delete_edges_or_change_menu_execution_permissions() {
    unsafe {
        let mut app = visible();
        app.add_window(false).unwrap();
        let catalog = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        let link = catalog.make_link(0, 1, 0, 0).unwrap().unwrap();
        app.apply_link_creation(desktop::link_catalog::Creation {
            catalog,
            link: link.clone(),
        });
        let unit = app.documents[0].state.unit_id.clone();
        app.workspace
            .set_port_visibility(&unit, Visibility::read(r#"{"output":false}"#).unwrap())
            .unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        assert_eq!(app.workspace.links(), std::slice::from_ref(&link));
        let scene = app.chrome.ui.scene.borrow().clone().unwrap();
        assert!(scene.catalog.permits(&link));
        assert!(!scene.catalog.endpoints[0].output_visibility[0]);
        let port = Port {
            node: 0,
            side: Side::Output,
            index: 0,
        };
        assert!(!app.chrome.ui.visible(port));
        assert!(app.chrome.ui.point(port).is_none());
        for (hwnd, node, side) in app.chrome.ui.input_windows.borrow().iter() {
            if *node == 0 && *side == Side::Output {
                assert!(!IsWindowVisible(*hwnd).as_bool());
            }
        }
        app.workspace
            .set_port_visibility(&unit, Visibility::default())
            .unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.visible(port));
        assert_eq!(app.workspace.links(), [link]);
        app.shutdown();
    }
}
#[test]
fn appearance_invalidates_save_stamp_and_restore_guard_but_noop_does_not() {
    unsafe {
        let mut app = super::super::tests::app();
        let stamp = crate::desktop::application::save_state::Stamp::capture(&app).unwrap();
        let guard =
            crate::desktop::application::restore_batch::guard::Guard::capture(&app).unwrap();
        let unit = app.documents[0].state.unit_id.clone();
        assert!(stamp.matches_model(&app) && guard.matches(&app));
        app.workspace
            .set_port_visibility(&unit, Visibility::default())
            .unwrap();
        assert!(stamp.matches_model(&app) && guard.matches(&app));
        app.workspace
            .set_port_visibility(&unit, Visibility::read(r#"{"output":false}"#).unwrap())
            .unwrap();
        assert!(!stamp.matches_model(&app) && !guard.matches(&app));
        app.shutdown();
    }
}
