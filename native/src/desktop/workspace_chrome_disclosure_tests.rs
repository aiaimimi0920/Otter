//! 主干逐行展开/Tab 边沿与 anchor 手势；只使用测试进程自有 HWND 和原事务。
use super::*;
use windows::Win32::{Graphics::Gdi::*, UI::Input::KeyboardAndMouse::*};

unsafe fn app() -> Application {
    let mut app = super::super::tests::app();
    app.add_window(false).unwrap();
    for (index, doc) in app.documents.iter().enumerate() {
        SetWindowPos(
            doc.window.0,
            None,
            360 + index as i32 * 400,
            140 + index as i32 * 280,
            240,
            220,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        let _ = ShowWindow(doc.window.0, SW_SHOWNOACTIVATE);
    }
    app.sync_link_menus();
    app.sync_workspace_chrome(true);
    app
}
fn port(node: usize, side: Side) -> Port {
    Port {
        node,
        side,
        index: 0,
    }
}
fn layout(app: &Application, port: Port) -> neuro_mini_ui::CapsuleLayout {
    port_visual::layout(&app.chrome.ui, port).unwrap()
}
unsafe fn mouse(app: &Application, port: Port, message: u32, point: Point) {
    let hwnd = app
        .chrome
        .ui
        .input_windows
        .borrow()
        .iter()
        .find(|(_, n, s)| *n == port.node && *s == port.side)
        .unwrap()
        .0;
    let mut local = POINT {
        x: point.x,
        y: point.y,
    };
    ScreenToClient(hwnd, &mut local).ok().unwrap();
    let lp = LPARAM((local.x as u16 as isize) | ((local.y as u16 as isize) << 16));
    SendMessageW(hwnd, message, None, Some(lp));
}
fn anchor(app: &Application, port: Port) -> Point {
    let (mut x, y) = layout(app, port).connection();
    // 收起标记仅外半边可点；不借图片/白框覆盖的内半边构造输入。
    x += if port.side == Side::Input { -8 } else { 8 };
    Point { x, y }
}
unsafe fn click(app: &Application, port: Port) {
    let p = anchor(app, port);
    mouse(app, port, WM_LBUTTONDOWN, p);
    mouse(app, port, WM_LBUTTONUP, p);
}
unsafe fn tab(app: &mut Application, node: usize) {
    app.documents[node].state.parameters_ui.toggle.set(true);
    app.process_parameter_panels(true);
    app.sync_workspace_chrome(true);
}

#[test]
fn tab_sets_all_rows_then_each_anchor_can_fold_and_reopen_without_changing_other_rows() {
    unsafe {
        let mut app = app();
        let a = port(0, Side::Output);
        let b = port(1, Side::Output);
        click(&app, b);
        tab(&mut app, 0);
        assert!(layout(&app, a).expanded && layout(&app, b).expanded);
        let input = port(0, Side::Input);
        assert!(layout(&app, input).expanded);
        click(&app, a);
        assert!(!layout(&app, a).expanded);
        assert!(layout(&app, input).expanded && layout(&app, b).expanded);
        let edge = app.chrome.ui.point(a);
        app.sync_workspace_chrome(true);
        assert_eq!(app.chrome.ui.point(a), edge);
        assert!(!layout(&app, a).expanded);
        click(&app, a);
        assert!(layout(&app, a).expanded);
        tab(&mut app, 0);
        assert!(!layout(&app, a).expanded && !layout(&app, input).expanded);
        assert!(layout(&app, b).expanded);
        assert!(!app.chrome.ui.disclosed.borrow().keys().any(|p| p.node == 0));
        click(&app, a);
        assert!(layout(&app, a).expanded);
        tab(&mut app, 0);
        assert!(layout(&app, a).expanded && layout(&app, input).expanded);
        click(&app, input);
        assert!(!layout(&app, input).expanded);
        assert!(layout(&app, a).expanded);
        app.shutdown();
    }
}

#[test]
fn a_locally_folded_hidden_row_disappears_until_the_next_tab_expansion_edge() {
    unsafe {
        let mut app = app();
        let unit = app.documents[0].state.unit_id.clone();
        app.workspace
            .set_port_visibility(
                &unit,
                crate::workspace::ports::Visibility::read(r#"{"output":false}"#).unwrap(),
            )
            .unwrap();
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        let target = port(0, Side::Output);
        assert!(app.chrome.ui.point(target).is_none());
        tab(&mut app, 0);
        assert!(layout(&app, target).expanded);
        click(&app, target);
        assert!(app.chrome.ui.point(target).is_none());
        app.sync_workspace_chrome(true);
        assert!(app.chrome.ui.point(target).is_none());
        tab(&mut app, 0);
        assert!(app.chrome.ui.point(target).is_none());
        tab(&mut app, 0);
        assert!(layout(&app, target).expanded);
        assert_eq!(
            app.workspace.port_visibility(&unit).get("output"),
            Some(false)
        );
        app.shutdown();
    }
}

#[test]
fn body_clicks_and_short_releases_outside_the_anchor_do_not_toggle_or_start_a_link() {
    unsafe {
        let mut app = app();
        let target = port(0, Side::Output);
        click(&app, target);
        let row = layout(&app, target);
        let body = Point {
            x: row.body.x + row.body.width / 4,
            y: row.body.y + row.body.height / 2,
        };
        mouse(&app, target, WM_LBUTTONDOWN, body);
        assert!(GetCapture().is_invalid());
        assert!(app.chrome.ui.press.borrow().is_none());
        mouse(&app, target, WM_LBUTTONUP, body);
        assert!(layout(&app, target).expanded);
        let start = Point {
            x: row.anchor.x + row.anchor.width - 2,
            y: body.y,
        };
        mouse(&app, target, WM_LBUTTONDOWN, start);
        assert!(!GetCapture().is_invalid());
        mouse(
            &app,
            target,
            WM_LBUTTONUP,
            Point {
                x: start.x + 3,
                ..start
            },
        );
        assert!(GetCapture().is_invalid());
        assert!(layout(&app, target).expanded);
        assert!(app.chrome.ui.pending.borrow().is_none());
        let center = layout(&app, target).connection();
        let start = Point {
            x: center.0,
            y: center.1,
        };
        mouse(&app, target, WM_LBUTTONDOWN, start);
        // 即使没有中间 WM_MOUSEMOVE，释放边沿的实际位移也须满足点击阈值。
        mouse(
            &app,
            target,
            WM_LBUTTONUP,
            Point {
                x: start.x + 6,
                ..start
            },
        );
        assert!(layout(&app, target).expanded);
        assert!(app.chrome.ui.pending.borrow().is_none());
        click(&app, target);
        assert!(!layout(&app, target).expanded);
        app.shutdown();
    }
}

#[test]
fn a_real_drop_on_a_body_is_rejected_but_the_same_drop_on_its_anchor_uses_the_graph_transaction() {
    desktop::workspace_window_fixture::with_private_desktop(|| unsafe {
        let mut app = app();
        let source = port(0, Side::Output);
        let target = port(1, Side::Input);
        click(&app, target);
        let body = layout(&app, target).body;
        let body = Point {
            x: body.x + body.width / 4,
            y: body.y + body.height / 2,
        };
        for accepted in [false, true] {
            let end = if accepted { anchor(&app, target) } else { body };
            for hwnd in app.chrome.handles() {
                UpdateWindow(hwnd).ok().unwrap();
            }
            let found = WindowFromPoint(POINT { x: end.x, y: end.y });
            let expected = app
                .chrome
                .ui
                .input_windows
                .borrow()
                .iter()
                .find(|(_, node, side)| *node == target.node && *side == target.side)
                .map(|(hwnd, _, _)| *hwnd)
                .expect("target strip must exist");
            assert_eq!(found, expected, "drop hit at ({}, {})", end.x, end.y);
            mouse(&app, source, WM_LBUTTONDOWN, anchor(&app, source));
            mouse(&app, source, WM_MOUSEMOVE, end);
            mouse(&app, source, WM_LBUTTONUP, end);
            assert_eq!(app.chrome.ui.pending.borrow().is_some(), accepted);
            app.pending_link_changes();
            assert_eq!(app.workspace.links().len(), usize::from(accepted));
        }
        app.shutdown();
    });
}

#[test]
fn keyboard_focus_does_not_expand_a_row_and_enter_space_really_toggle_its_geometry() {
    unsafe {
        let mut app = app();
        let target = port(0, Side::Output);
        keyboard::restore(&app.chrome.ui, target);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(target));
        assert!(!layout(&app, target).expanded);
        for key in [VK_RETURN, VK_SPACE] {
            assert!(app.translate_workspace_chrome(&MSG {
                hwnd: GetFocus(),
                message: WM_KEYDOWN,
                wParam: WPARAM(key.0 as usize),
                ..Default::default()
            }));
            assert_eq!(layout(&app, target).expanded, key == VK_RETURN);
        }
        tab(&mut app, 0);
        keyboard::restore(&app.chrome.ui, target);
        assert!(layout(&app, target).expanded);
        assert!(app.translate_workspace_chrome(&MSG {
            hwnd: GetFocus(),
            message: WM_KEYDOWN,
            wParam: WPARAM(VK_RETURN.0 as usize),
            ..Default::default()
        }));
        assert!(!layout(&app, target).expanded);
        assert_eq!(app.chrome.ui.keyboard.focused(), Some(target));
        app.shutdown();
    }
}

#[test]
fn both_local_states_rebind_by_field_but_not_across_host_identity_or_tab_edges() {
    unsafe {
        let mut app = app();
        let ui = &app.chrome.ui;
        ui.disclosed.borrow_mut().insert(port(0, Side::Input), true);
        ui.disclosed
            .borrow_mut()
            .insert(port(0, Side::Output), false);
        ui.disclosed
            .borrow_mut()
            .insert(port(1, Side::Output), true);
        let mut next = Catalog::capture(&app.documents, &app.live_sessions, &app.workspace);
        Rc::get_mut(&mut next).unwrap().endpoints[0]
            .inputs
            .insert(0, "new-field".into());
        let mut nodes = ui.nodes.borrow().clone();
        sync(ui, &next, &nodes);
        let shifted = Port {
            index: 1,
            ..port(0, Side::Input)
        };
        assert_eq!(ui.disclosed.borrow().get(&shifted), Some(&true));
        assert_eq!(
            ui.disclosed.borrow().get(&port(0, Side::Output)),
            Some(&false)
        );
        assert_eq!(ui.disclosed.borrow().len(), 3);
        *ui.scene.borrow_mut() = Some(Rc::new(scene::Scene {
            catalog: next.clone(),
            edges: Vec::new(),
            wake: HWND::default(),
        }));
        nodes.iter_mut().find(|n| n.endpoint == 0).unwrap().expanded = true;
        sync(ui, &next, &nodes);
        assert_eq!(ui.disclosed.borrow().len(), 1);
        assert_eq!(
            ui.disclosed.borrow().get(&port(1, Side::Output)),
            Some(&true)
        );
        *ui.nodes.borrow_mut() = nodes.clone();
        nodes.iter_mut().find(|n| n.endpoint == 1).unwrap().marker += 1;
        sync(ui, &next, &nodes);
        assert!(ui.disclosed.borrow().is_empty());
        app.shutdown();
    }
}
