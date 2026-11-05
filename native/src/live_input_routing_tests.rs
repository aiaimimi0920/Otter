use super::*;
use windows::{
    core::w,
    Win32::UI::{HiDpi::*, Input::KeyboardAndMouse::EnableWindow},
};
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
fn window(
    parent: Option<HWND>,
    style: WINDOW_STYLE,
    ex: WINDOW_EX_STYLE,
    point: [i32; 2],
    size: i32,
) -> Window {
    Window(
        unsafe {
            CreateWindowExW(
                ex,
                w!("STATIC"),
                w!(""),
                style,
                point[0],
                point[1],
                size,
                size,
                parent,
                None,
                None,
                None,
            )
        }
        .unwrap(),
    )
}
#[test]
#[ignore = "requires interactive default desktop; hit-tests only owned hidden windows without input"]
fn deep_hit_skips_unavailable_children_preserves_transparency_and_retains_gesture() {
    unsafe {
        let root = window(
            None,
            WS_POPUP,
            WS_EX_LAYERED | WS_EX_TRANSPARENT,
            [100, 100],
            200,
        );
        SetLayeredWindowAttributes(root.0, COLORREF(0), 1, LWA_ALPHA).unwrap();
        let child = window(
            Some(root.0),
            WS_CHILD | WS_VISIBLE,
            WINDOW_EX_STYLE::default(),
            [20, 20],
            100,
        );
        let nested = window(
            Some(child.0),
            WS_CHILD | WS_VISIBLE,
            WINDOW_EX_STYLE::default(),
            [10, 10],
            50,
        );
        let target = Target::bind(root.0).unwrap();
        // 生产隐藏采用 alpha=1 而非 SW_HIDE；CWP_SKIPINVISIBLE 应跳过真正隐藏的子树。
        assert_eq!(pointer(&target, [35, 35]).unwrap().hwnd(), root.0);
        let _ = ShowWindow(root.0, SW_SHOWNOACTIVATE);
        let mut exhausted = 0;
        let mut screen = POINT { x: 35, y: 35 };
        assert!(ClientToScreen(root.0, &mut screen).as_bool());
        assert_eq!(
            child_at(root.0, screen, &mut exhausted),
            Err(Error::DepthLimit)
        );
        let style = GetWindowLongPtrW(root.0, GWL_EXSTYLE);
        let dpi = GetThreadDpiAwarenessContext();
        let hit = pointer(&target, [35, 35]).unwrap();
        eprintln!("routing-fixture: root={:?} child={:?} nested={:?} hit={:?} visible={:?}/{:?}/{:?} raw={:?}", root.0, child.0, nested.0, hit.hwnd(), IsWindowVisible(root.0), IsWindowVisible(child.0), IsWindowVisible(nested.0), ChildWindowFromPointEx(root.0, POINT { x: 35, y:35 }, CWP_SKIPDISABLED | CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT));
        assert_eq!(hit.hwnd(), nested.0);
        assert_eq!(GetWindowLongPtrW(root.0, GWL_EXSTYLE), style);
        assert!(AreDpiAwarenessContextsEqual(dpi, GetThreadDpiAwarenessContext()).as_bool());
        assert_eq!(retained(&target, &hit).unwrap().hwnd(), nested.0);
        let _ = EnableWindow(nested.0, false);
        assert_eq!(pointer(&target, [35, 35]).unwrap().hwnd(), child.0);
        let _ = EnableWindow(nested.0, true);
        SetWindowLongPtrW(nested.0, GWL_EXSTYLE, WS_EX_TRANSPARENT.0 as isize);
        assert_eq!(pointer(&target, [35, 35]).unwrap().hwnd(), child.0);
        SetWindowLongPtrW(nested.0, GWL_EXSTYLE, 0);
        let _ = ShowWindow(nested.0, SW_HIDE);
        assert_eq!(pointer(&target, [35, 35]).unwrap().hwnd(), child.0);
        assert_eq!(pointer(&target, [190, 190]).unwrap().hwnd(), root.0);
        assert_eq!(keyboard(&target).unwrap().hwnd(), root.0);
        drop(nested);
        assert_eq!(retained(&target, &hit).unwrap().hwnd(), root.0);
        drop(child);
        drop(root);
        assert!(retained(&target, &hit).is_err());
        assert!(pointer(&target, [35, 35]).is_err());
    }
}
