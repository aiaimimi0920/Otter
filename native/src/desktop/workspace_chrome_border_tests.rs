//! 单宿主外框的透明度、层序、显隐与拖动合成帧交接；只使用隔离窗口。
use super::*;

#[test]
fn pure_z_order_changes_keep_each_border_immediately_above_its_host() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let mut observed = Vec::new();
        for upper in [0, 1, 0, 1] {
            let host = app.documents[upper].window.0;
            SetWindowPos(
                host,
                Some(HWND_TOP),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .unwrap();
            let top_border = border_handle(&ui, upper);
            assert_eq!(GetWindow(top_border, GW_HWNDNEXT).unwrap(), host);
            let mut chain = Vec::new();
            let mut entry = GetWindow(host, GW_HWNDFIRST).unwrap_or_default();
            for _ in 0..128 {
                if entry.is_invalid() {
                    break;
                }
                chain.push(entry);
                entry = GetWindow(entry, GW_HWNDNEXT).unwrap_or_default();
            }
            // HWND_TOPMOST 改变置顶类别，不作为两宿主实际顺序的测试替身。
            // 根据真实链检查每个框都紧邻宿主；另记录宿主确实发生过顺序交换。
            let rank = |hwnd| chain.iter().position(|entry| *entry == hwnd).unwrap();
            for endpoint in 0..2 {
                assert_eq!(
                    rank(border_handle(&ui, endpoint)) + 1,
                    rank(app.documents[endpoint].window.0),
                    "chain={chain:?}"
                );
            }
            observed.push(rank(app.documents[0].window.0) < rank(app.documents[1].window.0));
        }
        assert!(
            observed.contains(&true) && observed.contains(&false),
            "{observed:?}"
        );
        app.shutdown();
    }
}

#[test]
fn opacity_changes_only_its_border_and_modes_keep_ports_independent() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let first = border_handle(&ui, 0);
        let second = border_handle(&ui, 1);
        for (opacity, expected) in [(0.0, 128), (0.3, 166), (0.5, 191), (1.0, 255)] {
            app.documents[0].state.canvas.borrow_mut().window_opacity = opacity;
            app.sync_workspace_chrome(true);
            let borders = ui.borders.borrow();
            assert_eq!(
                borders.iter().find(|b| b.hwnd == first).unwrap().alpha(),
                expected
            );
            assert_eq!(
                borders.iter().find(|b| b.hwnd == second).unwrap().alpha(),
                255
            );
            drop(borders);
            assert!(IsWindowVisible(first).as_bool() && IsWindowVisible(second).as_bool());
            let rect = ui.node(0).unwrap().rect;
            assert!(
                !border_covers(
                    &ui,
                    0,
                    Point {
                        x: rect.left,
                        y: rect.top
                    }
                ),
                "border must not cover own content"
            );
        }
        for mode in [Mode::None, Mode::Border, Mode::All] {
            ui.mode.set(mode);
            window::refresh(&ui).unwrap();
            assert_eq!(IsWindowVisible(first).as_bool(), mode != Mode::None);
            assert_eq!(IsWindowVisible(second).as_bool(), mode != Mode::None);
        }
        app.shutdown();
        assert!(!IsWindow(Some(first)).as_bool() && !IsWindow(Some(second)).as_bool());
        assert!(ui.borders.borrow().is_empty());
    }
}

#[test]
fn moving_pin_uses_one_cached_image_border_window_and_restores_only_its_border() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let doc = &app.documents[0];
        {
            let mut canvas = doc.state.canvas.borrow_mut();
            canvas.pinned = true;
            canvas.tool = crate::editor::Tool::Pan;
            canvas
                .replace_image(desktop::image::Image {
                    width: 8,
                    height: 8,
                    pixels: [20, 80, 160, 255].repeat(64).into(),
                })
                .unwrap();
        }
        desktop::pin_window::fit(doc.window.0, &doc.state);
        let first = border_handle(&ui, 0);
        let other = border_handle(&ui, 1);
        let before = doc.state.canvas.borrow().paints;
        SendMessageW(doc.window.0, WM_ENTERSIZEMOVE, None, None);
        let proxy = desktop::pin_window::preview::handle(&doc.state).unwrap();
        assert_eq!(proxy, first);
        assert!(IsWindowVisible(first).as_bool());
        assert!(IsWindowVisible(other).as_bool());
        let padding = desktop::pin_border_style::width(windows::Win32::UI::HiDpi::GetDpiForWindow(
            doc.window.0,
        )) as i32;
        for index in 0..100 {
            let (x, y) = (120 + index * 5, 140 + index % 3 * 70);
            SetWindowPos(
                doc.window.0,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            let mut rect = RECT::default();
            GetWindowRect(proxy, &mut rect).unwrap();
            assert_eq!(
                rect,
                RECT {
                    left: x - padding,
                    top: y - padding,
                    right: x + 160 + padding,
                    bottom: y + 120 + padding
                }
            );
            assert!(IsWindowVisible(first).as_bool());
            assert!(IsWindowVisible(other).as_bool());
            assert_eq!(
                doc.state.canvas.borrow().paints,
                before,
                "drag must not repaint original backend each move"
            );
        }
        SendMessageW(doc.window.0, WM_EXITSIZEMOVE, None, None);
        SendMessageW(doc.window.0, WM_CANCELMODE, None, None);
        assert!(IsWindow(Some(proxy)).as_bool());
        assert!(desktop::pin_window::preview::handle(&doc.state).is_none());
        assert!(IsWindowVisible(first).as_bool() && IsWindowVisible(other).as_bool());
        assert!(doc.state.error.borrow().is_none());
        assert_eq!(GetWindow(first, GW_HWNDNEXT).unwrap(), doc.window.0);
        app.shutdown();
    }
}
