//! PMv2 父窗口托管 unaware 子控件；覆盖控件外拖动，不修改全局 DPI 设置。
use super::*;
use windows::core::w;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
struct Hosting(DPI_HOSTING_BEHAVIOR);
impl Drop for Hosting {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiHostingBehavior(self.0);
        }
    }
}

#[test]
#[ignore = "interactive desktop; own mixed-hosting parent and child, no input"]
fn actual_mixed_awareness_child_accepts_inside_and_outside_drag_coordinates() {
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let old_hosting = SetThreadDpiHostingBehavior(DPI_HOSTING_BEHAVIOR_MIXED);
        assert_ne!(old_hosting, DPI_HOSTING_BEHAVIOR_INVALID);
        let hosting = Hosting(old_hosting);
        let root = Window(
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Otter mixed DPI fixture"),
                WS_POPUP,
                100,
                100,
                300,
                200,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        drop(hosting);
        let child = {
            let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_UNAWARE);
            assert!(!old.0.is_null());
            let _dpi = Dpi(old);
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                20,
                10,
                80,
                40,
                Some(root.0),
                None,
                None,
                None,
            )
            .unwrap()
        };
        assert_eq!(
            GetAwarenessFromDpiAwarenessContext(GetWindowDpiAwarenessContext(child)),
            DPI_AWARENESS_UNAWARE
        );
        SetLayeredWindowAttributes(root.0, COLORREF(0), 1, LWA_ALPHA).unwrap();
        let _ = ShowWindow(root.0, SW_SHOWNOACTIVATE);
        let grandchild = {
            let _dpi = Dpi::enter(child).unwrap();
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD | WS_VISIBLE,
                60,
                20,
                15,
                15,
                Some(child),
                None,
                None,
                None,
            )
            .unwrap()
        };
        let target = Target::bind(root.0).unwrap();
        let mut grand_bounds = RECT::default();
        GetWindowRect(grandchild, &mut grand_bounds).unwrap();
        let mut hit = POINT {
            x: (grand_bounds.left + grand_bounds.right) / 2,
            y: (grand_bounds.top + grand_bounds.bottom) / 2,
        };
        assert!(ScreenToClient(root.0, &mut hit).as_bool());
        let routed = super::super::input_routing::pointer(&target, [hit.x, hit.y]).unwrap();
        assert_eq!(
            routed.hwnd(),
            grandchild,
            "mixed-DPI nested child must remain reachable"
        );
        let destination = Destination::bind(&target, child).unwrap();
        let mut bounds = RECT::default();
        GetWindowRect(child, &mut bounds).unwrap();
        let client = {
            let _dpi = Dpi::enter(child).unwrap();
            let mut client = RECT::default();
            GetClientRect(child, &mut client).unwrap();
            client
        };
        let before = GetThreadDpiAwarenessContext();
        for point in [[0.2, 0.15], [0.0, 0.0], [1.0, 1.0], [0.5, 0.5]] {
            let mapped = map(&target, &destination, point, None)
                .unwrap_or_else(|error| panic!("point={point:?}: {error:?}"));
            let mut screen = POINT {
                x: mapped.root_client[0],
                y: mapped.root_client[1],
            };
            assert!(ClientToScreen(root.0, &mut screen).as_bool());
            let expected = [
                ((screen.x - bounds.left) as f64 * client.right as f64
                    / (bounds.right - bounds.left) as f64)
                    .round() as i32,
                ((screen.y - bounds.top) as f64 * client.bottom as f64
                    / (bounds.bottom - bounds.top) as f64)
                    .round() as i32,
            ];
            for axis in 0..2 {
                assert!(
                    mapped.destination_client[axis].abs_diff(expected[axis]) <= 1,
                    "point={point:?} mapped={mapped:?} expected={expected:?}"
                );
            }
            assert!(AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool());
        }
        eprintln!(
            "native-mixed-dpi: root={} child={} inside=true outside=true",
            GetDpiForWindow(root.0),
            GetDpiForWindow(child)
        );
    }
}
