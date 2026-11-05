//! Chromium range 混合输入对照：键盘提交重置拖动基线，Tab 提交未结束手势。
use super::*;
#[test]
fn surface_slider_mixed_focus_commit_callback_may_destroy_control() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    parent.log.close_on_change = true;
    unsafe {
        mouse(hwnd, WM_LBUTTONDOWN, 0);
        let _ = SetFocus(Some(parent.hwnd));
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert_ne!(GetCapture(), hwnd);
    }
    assert_eq!(parent.log.events, [(0, 0.0), (1, 0.0)]);
}
#[test]
fn surface_slider_mixed_boundary_key_does_not_commit_pending_drag_early() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (_, right, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, right);
        key(hwnd, VK_RIGHT);
        assert_eq!(parent.log.events, [(0, 100.0)]);
        mouse(hwnd, WM_LBUTTONUP, right);
    }
    assert_eq!(parent.log.events, [(0, 100.0), (1, 100.0)]);
}
#[test]
fn surface_slider_mixed_tab_commit_callback_may_destroy_control() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    parent.log.close_on_change = true;
    unsafe {
        mouse(hwnd, WM_LBUTTONDOWN, 0);
        key(hwnd, VK_TAB);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert_ne!(GetCapture(), hwnd);
    }
    assert_eq!(parent.log.events, [(0, 0.0), (1, 0.0)]);
}
#[test]
fn surface_slider_mixed_key_commit_does_not_commit_again_on_release() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (left, _, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        key(hwnd, VK_RIGHT);
        assert_eq!(GetCapture(), hwnd);
        mouse(hwnd, WM_LBUTTONUP, left);
    }
    assert_eq!(parent.log.events, [(0, 0.0), (0, 1.0), (1, 1.0)]);
}
#[test]
fn surface_slider_mixed_drag_after_key_commit_still_commits_new_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (left, right, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        key(hwnd, VK_RIGHT);
        mouse(hwnd, WM_MOUSEMOVE, right);
        mouse(hwnd, WM_LBUTTONUP, right);
    }
    assert_eq!(
        parent.log.events,
        [(0, 0.0), (0, 1.0), (1, 1.0), (0, 100.0), (1, 100.0)]
    );
}
#[test]
fn surface_slider_mixed_tab_commits_drag_once_and_keeps_capture_until_up() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (left, _, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        key(hwnd, VK_TAB);
        assert_eq!(GetCapture(), hwnd);
        mouse(hwnd, WM_LBUTTONUP, left);
        assert_ne!(GetCapture(), hwnd);
    }
    assert_eq!(parent.log.events, [(0, 0.0), (1, 0.0)]);
}
#[test]
fn surface_slider_mixed_escape_preserves_drag_until_release() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (left, _, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        key(hwnd, VK_ESCAPE);
        assert_eq!(GetCapture(), hwnd);
        assert_eq!(parent.log.events, [(0, 0.0)]);
        mouse(hwnd, WM_LBUTTONUP, left);
    }
    assert_eq!(parent.log.events, [(0, 0.0), (1, 0.0)]);
}
