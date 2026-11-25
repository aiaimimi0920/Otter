//! Endpoints/curves form one connection group: actual Win32 Z-order, focus, visibility, and node reclamation using isolated owned windows only.
use super::*;

unsafe fn pin_order(ui: &Ui) -> Vec<isize> {
    let hosts: HashSet<_> = ui.nodes.borrow().iter().map(|node| node.hwnd).collect();
    let mut order = Vec::new();
    let mut cursor = GetTopWindow(None).unwrap();
    for _ in 0..4096 {
        if cursor.is_invalid() {
            break;
        }
        if hosts.contains(&(cursor.0 as isize)) {
            order.push(cursor.0 as isize);
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
    }
    assert_eq!(order.len(), hosts.len());
    order
}

unsafe fn rank(hwnd: HWND) -> usize {
    let mut cursor = GetTopWindow(None).unwrap();
    for index in 0..4096 {
        if cursor == hwnd {
            return index;
        }
        cursor = GetWindow(cursor, GW_HWNDNEXT).unwrap_or_default();
        if cursor.is_invalid() {
            break;
        }
    }
    panic!("missing test window: {hwnd:?}");
}

#[test]
fn tab_sides_follow_their_pin_across_an_independent_window_and_topmost_changes() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        for doc in &app.documents {
            doc.state.parameters_ui.toggle.set(true);
        }
        app.process_parameter_panels(true);
        app.sync_workspace_chrome(true);
        let foreign = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("stack fixture"),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            40,
            40,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let lower = app.documents[1].window.0;
        let upper = app.documents[0].window.0;
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
        SetWindowPos(lower, Some(HWND_BOTTOM), 0, 0, 0, 0, flags).unwrap();
        SetWindowPos(foreign, Some(HWND_TOP), 0, 0, 0, 0, flags).unwrap();
        for band in [HWND_TOPMOST, HWND_NOTOPMOST, HWND_TOPMOST] {
            SetWindowPos(upper, Some(band), 0, 0, 0, 0, flags).unwrap();
            assert!(rank(upper) < rank(foreign) && rank(foreign) < rank(lower));
            for side in [Side::Input, Side::Output] {
                let (attached, _, _) = port(&ui, 0, side);
                assert!(IsWindowVisible(attached).as_bool());
                assert!(
                    rank(attached) < rank(foreign),
                    "Tab {side:?} must follow its own pin, not the lowest pin"
                );
                assert_eq!(
                    GetWindowLongPtrW(attached, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                    GetWindowLongPtrW(upper, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0
                );
            }
            let node = ui.node(0).unwrap();
            let backing = ui
                .backings
                .borrow()
                .iter()
                .find(|b| b.matches(&node))
                .unwrap()
                .hwnd;
            assert!(
                rank(backing) < rank(foreign),
                "Tab row backing must follow the same pin"
            );
            atomic_tests::links_below_nodes(&ui);
        }
        DestroyWindow(foreign).unwrap();
        app.shutdown();
    }
}

#[test]
fn port_focus_preview_and_reshow_keep_the_entire_connection_group_below_unchanged_pins() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        for endpoint in [0, 1, 0] {
            let (strip, port, point) = port(&ui, endpoint, Side::Output);
            let _ = SetFocus(Some(app.documents[endpoint].window.0));
            // Finish fixture-driven host selection first; endpoint focus afterward must not alter relative image order.
            let before = pin_order(&ui);
            let _ = SetFocus(Some(strip));
            assert_eq!(GetFocus(), strip);
            atomic_tests::links_below_nodes(&ui);
            keyboard::restore(&ui, port);
            assert_eq!(GetFocus(), strip);
            atomic_tests::links_below_nodes(&ui);
            assert_eq!(pin_order(&ui), before);
            mouse(&ui, strip, WM_LBUTTONDOWN, point);
            mouse(
                &ui,
                strip,
                WM_MOUSEMOVE,
                Point {
                    x: point.x + 60,
                    y: point.y + 30,
                },
            );
            assert!(ui.press.borrow().as_ref().unwrap().dragged);
            atomic_tests::links_below_nodes(&ui);
            ui.cancel();
            for mode in [Mode::None, Mode::Border, Mode::All] {
                ui.mode.set(mode);
                window::refresh(&ui).unwrap();
                atomic_tests::links_below_nodes(&ui);
            }
            window::hide(&ui);
            assert!(ui.handles().iter().all(|h| !IsWindowVisible(*h).as_bool()));
            window::refresh(&ui).unwrap();
            atomic_tests::links_below_nodes(&ui);
            assert!(ui.failure.borrow().is_none());
        }
        app.shutdown();
    }
}

#[test]
fn embedded_child_focus_keeps_ports_and_curves_below_pins_without_waiting_for_refresh() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let (strip, _, _) = port(&ui, 0, Side::Input);
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            0,
            0,
            20,
            20,
            Some(strip),
            None,
            Some(HINSTANCE(
                windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                    .unwrap()
                    .0,
            )),
            None,
        )
        .unwrap();
        let _ = SetFocus(Some(app.documents[0].window.0));
        let before = pin_order(&ui);
        let _ = SetFocus(Some(child));
        assert_eq!(GetFocus(), child);
        atomic_tests::links_below_nodes(&ui);
        assert_eq!(pin_order(&ui), before);
        assert!(ui.failure.borrow().is_none());
        DestroyWindow(child).unwrap();
        app.shutdown();
    }
}

#[test]
fn a_raised_port_is_lowered_with_curves_even_when_the_curve_window_is_already_below_pins() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let before = pin_order(&ui);
        let (strip, _, _) = port(&ui, 0, Side::Input);
        SetWindowPos(
            strip,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )
        .unwrap();
        window::refresh(&ui).unwrap();
        atomic_tests::links_below_nodes(&ui);
        assert_eq!(pin_order(&ui), before);
        app.shutdown();
    }
}

#[test]
fn one_order_transaction_demotes_every_topmost_member_without_reordering_pins() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let before = pin_order(&ui);
        let layers: Vec<_> = ui
            .input_windows
            .borrow()
            .iter()
            .map(|(h, _, _)| *h)
            .chain(ui.backings.borrow().iter().map(|b| b.hwnd))
            .chain([ui.drawing.get()])
            .collect();
        for mask in 1..1usize << layers.len() {
            // Temporarily suppress synchronous correction in the fixture to create a complete batch with differing previous TOPMOST states.
            assert!(!ui.ordering.replace(true));
            for (index, hwnd) in layers.iter().enumerate() {
                if mask & (1 << index) != 0 {
                    SetWindowPos(
                        *hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                    )
                    .unwrap();
                }
            }
            ui.ordering.set(false);
            window::order_drawing(&ui).unwrap();
            atomic_tests::links_below_nodes(&ui);
            assert_eq!(pin_order(&ui), before);
            for hwnd in &layers {
                assert_eq!(
                    GetWindowLongPtrW(*hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                    0,
                    "mask={mask}: stale topmost state must not propagate to another member"
                );
            }
        }
        assert!(ui.failure.borrow().is_none());
        app.shutdown();
    }
}

#[test]
fn a_legal_shared_curve_layer_is_not_raised_above_an_independent_window() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let lowest = *pin_order(&ui).last().unwrap();
        let node = ui
            .nodes
            .borrow()
            .iter()
            .find(|n| n.hwnd == lowest)
            .unwrap()
            .clone();
        let tail = ui
            .backings
            .borrow()
            .iter()
            .find(|b| b.matches(&node))
            .unwrap()
            .hwnd;
        let foreign = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("curve fixture"),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            40,
            40,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        SetWindowPos(
            foreign,
            Some(tail),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )
        .unwrap();
        assert!(rank(foreign) < rank(ui.drawing.get()));
        let before = pin_order(&ui);
        window::refresh(&ui).unwrap();
        assert!(
            rank(foreign) < rank(ui.drawing.get()),
            "legal shared layer must not be raised"
        );
        assert_eq!(pin_order(&ui), before);
        atomic_tests::links_below_nodes(&ui);
        DestroyWindow(foreign).unwrap();
        app.shutdown();
    }
}

#[test]
fn closing_a_pin_hides_its_ports_immediately_without_destroying_the_other_connection_group() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let drawing = ui.drawing.get();
        let inputs = ui.input_windows.borrow().clone();
        let closed: Vec<_> = inputs
            .iter()
            .filter(|(_, n, _)| *n == 0)
            .map(|(h, _, _)| *h)
            .collect();
        let retained: Vec<_> = inputs
            .iter()
            .filter(|(_, n, _)| *n == 1)
            .map(|(h, _, _)| *h)
            .collect();
        let host = app.documents[0].window.0;
        keyboard::restore(&ui, port(&ui, 0, Side::Input).1);
        DestroyWindow(host).unwrap();
        assert!(ui.node(0).is_none());
        assert!(ui.keyboard.focused().is_none());
        assert!(closed.iter().all(|h| !IsWindowVisible(*h).as_bool()));
        assert!(retained.iter().all(|h| IsWindowVisible(*h).as_bool()));
        assert!(IsWindowVisible(drawing).as_bool());
        app.sync_link_menus();
        app.sync_workspace_chrome(true);
        // Single-host destruction already removed nodes synchronously; explicitly sync resource owners rather than rely on application polling/pruning.
        app.chrome.windows.as_mut().unwrap().sync().unwrap();
        assert!(closed.iter().all(|h| !IsWindow(Some(*h)).as_bool()));
        assert!(retained.iter().all(|h| IsWindowVisible(*h).as_bool()));
        atomic_tests::links_below_nodes(&ui);
        let _ = ShowWindow(app.documents[1].window.0, SW_HIDE);
        assert!(retained.iter().all(|h| !IsWindowVisible(*h).as_bool()));
        let _ = ShowWindow(app.documents[1].window.0, SW_SHOWNOACTIVATE);
        assert!(retained.iter().all(|h| IsWindowVisible(*h).as_bool()));
        atomic_tests::links_below_nodes(&ui);
        assert!(ui.failure.borrow().is_none());
        app.shutdown();
        assert!(!IsWindow(Some(drawing)).as_bool());
        assert!(retained.iter().all(|h| !IsWindow(Some(*h)).as_bool()));
    }
}
