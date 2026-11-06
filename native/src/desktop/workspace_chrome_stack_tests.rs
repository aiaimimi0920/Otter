//! 端点/曲线同属连接组：真实 Win32 层序、焦点、显隐和节点回收；只操作隔离自有窗口。
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

#[test]
fn port_focus_preview_and_reshow_keep_the_entire_connection_group_below_unchanged_pins() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        for endpoint in [0, 1, 0] {
            let (strip, port, point) = port(&ui, endpoint, Side::Output);
            let _ = SetFocus(Some(app.documents[endpoint].window.0));
            // 先完成夹具主动选择宿主；之后端点获取焦点不得再改变图片相对顺序。
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
        // 单个宿主销毁已同步移出 nodes；显式执行资源 owner 的 sync，不依赖应用轮询剪枝。
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
