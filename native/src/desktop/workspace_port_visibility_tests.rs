//! Actual owned port-HWND expansion/eye buttons/single-slot commits, executed only on isolated desktops never switched to.
use super::*;
use windows::Win32::{Graphics::Gdi::ScreenToClient, UI::Input::KeyboardAndMouse::*};
fn target(side: Side) -> Port {
    Port {
        node: 0,
        side,
        index: 0,
    }
}
fn strip(app: &Application, side: Side) -> HWND {
    app.chrome
        .ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == 0 && *s == side)
        .unwrap()
        .0
}
unsafe fn mouse(app: &Application, side: Side, message: u32, point: Point) {
    let hwnd = strip(app, side);
    let mut p = POINT {
        x: point.x,
        y: point.y,
    };
    ScreenToClient(hwnd, &mut p).ok().unwrap();
    let lp = LPARAM((p.x as u16 as isize) | ((p.y as u16 as isize) << 16));
    SendMessageW(hwnd, message, None, Some(lp));
}
unsafe fn expand(app: &Application, side: Side) {
    let port = target(side);
    let mut p = app.chrome.ui.point(port).unwrap();
    p.x += if side == Side::Input { -8 } else { 8 };
    mouse(app, side, WM_LBUTTONDOWN, p);
    mouse(app, side, WM_LBUTTONUP, p);
    assert_eq!(app.chrome.ui.disclosed.borrow().get(&port), Some(&true));
}
fn action(app: &Application, side: Side) -> Point {
    let ui = &app.chrome.ui;
    let f = port_visual::layout(ui, target(side))
        .unwrap()
        .inline_action(ui.node(0).unwrap().dpi)
        .unwrap();
    Point {
        x: f.x + f.width / 2,
        y: f.y + f.height / 2,
    }
}
unsafe fn sync(app: &mut Application) {
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    for h in app.chrome.handles() {
        windows::Win32::Graphics::Gdi::UpdateWindow(h).ok().unwrap();
    }
}
#[test]
fn actual_eye_click_changes_only_visibility_and_keeps_the_open_row_and_connection_anchor() {
    unsafe {
        for side in [Side::Input, Side::Output] {
            let mut app = visible();
            expand(&app, side);
            let handles = app.chrome.handles();
            let before = app.chrome.ui.point(target(side));
            let unit = app.documents[0].state.unit_id.clone();
            let topology = app.workspace.topology_stamp();
            let config = desktop::configuration_stamp::Stamp::capture(&app.documents[0].state);
            let save = desktop::application::save_state::Stamp::capture(&app).unwrap();
            let name = if side == Side::Input {
                "image"
            } else {
                "output"
            };
            for expected in [false, true] {
                let p = action(&app, side);
                mouse(&app, side, WM_LBUTTONDOWN, p);
                assert_eq!(GetCapture(), strip(&app, side));
                mouse(&app, side, WM_LBUTTONUP, p);
                assert!(GetCapture().is_invalid());
                assert!(app.chrome.ui.pending.borrow().is_none());
                assert!(app.chrome.ui.port_settings.borrow().is_some());
                app.process_port_settings();
                sync(&mut app);
                assert_eq!(
                    app.workspace.port_visibility(&unit).get(name),
                    Some(expected)
                );
                assert_eq!(app.chrome.ui.point(target(side)), before);
                assert_eq!(
                    app.chrome.ui.disclosed.borrow().get(&target(side)),
                    Some(&true)
                );
                assert!(
                    topology.matches(&app.workspace) && config.matches(&app.documents[0].state)
                );
                assert_eq!(app.chrome.handles(), handles);
            }
            assert!(!save.matches_model(&app));
            app.shutdown();
        }
    }
}
#[test]
fn hidden_rows_return_with_tab_expansion_and_collapsed_rows_hide_without_deleting_the_link() {
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
            .set_port_visibility(
                &unit,
                Visibility::read(r#"{"image":false,"output":false}"#).unwrap(),
            )
            .unwrap();
        sync(&mut app);
        assert!(app.chrome.ui.point(target(Side::Input)).is_none());
        assert!(app.chrome.ui.point(target(Side::Output)).is_none());
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        sync(&mut app);
        assert!(app.chrome.ui.node(0).unwrap().expanded);
        for side in [Side::Input, Side::Output] {
            let p = action(&app, side);
            assert!(port_visual::action_hit(&app.chrome.ui, target(side), p));
            assert!(IsWindowVisible(strip(&app, side)).as_bool());
        }
        app.documents[0].state.parameters_ui.toggle.set(true);
        app.process_parameter_panels(true);
        sync(&mut app);
        assert!(app.chrome.ui.point(target(Side::Input)).is_none());
        assert!(app.chrome.ui.point(target(Side::Output)).is_none());
        assert_eq!(app.workspace.links(), [link]);
        app.shutdown();
    }
}
#[test]
fn dragged_or_released_outside_eye_does_not_toggle_disclosure_or_start_linking() {
    unsafe {
        let mut app = visible();
        expand(&app, Side::Output);
        let p = action(&app, Side::Output);
        mouse(&app, Side::Output, WM_LBUTTONDOWN, p);
        mouse(
            &app,
            Side::Output,
            WM_MOUSEMOVE,
            Point {
                x: p.x + 30,
                y: p.y,
            },
        );
        mouse(&app, Side::Output, WM_LBUTTONUP, p);
        assert!(app.chrome.ui.port_settings.borrow().is_none());
        assert!(app.chrome.ui.pending.borrow().is_none());
        assert_eq!(
            app.chrome.ui.disclosed.borrow().get(&target(Side::Output)),
            Some(&true)
        );
        let unit = &app.documents[0].state.unit_id;
        assert!(app.workspace.port_visibility(unit).is_empty());
        app.shutdown();
    }
}
#[test]
fn queued_visibility_rejects_changed_owner_catalog_collapse_and_global_gates() {
    unsafe {
        for invalid in 0..7 {
            let mut app = visible();
            expand(&app, Side::Output);
            visibility::request(&app.chrome.ui, target(Side::Output));
            let unit = app.documents[0].state.unit_id.clone();
            let hwnd = app.documents[0].window.0;
            let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            match invalid {
                0 => app.documents[0].state.group_locked.set(true),
                1 => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                2 => {
                    app.workspace
                        .set_port_visibility(&unit, Visibility::read(r#"{"image":false}"#).unwrap())
                        .unwrap();
                }
                3 => app.capture_active.set(true),
                4 => {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, marker + 1);
                }
                5 => app.chrome.ui.disclosed.borrow_mut().clear(),
                6 => app.exit_requested.set(true),
                _ => unreachable!(),
            }
            app.process_port_settings();
            assert_eq!(app.workspace.port_visibility(&unit).get("output"), None);
            assert!(app.chrome.ui.port_settings.borrow().is_none());
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, marker);
            app.capture_active.set(false);
            app.exit_requested.set(false);
            app.shutdown();
        }
    }
}
