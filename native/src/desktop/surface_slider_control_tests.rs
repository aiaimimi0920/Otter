//! 真实自有 HWND 的消息/值/捕获/重入销毁；不注入全局鼠标键盘输入或发送业务网络。
use super::*;
use windows::Win32::UI::Shell::*;
#[path = "surface_slider_mixed_tests.rs"]
mod mixed;
#[path = "surface_slider_mount_tests.rs"]
mod mounted;
#[path = "surface_slider_paint_tests.rs"]
mod pixels;
#[path = "surface_slider_touch_probe_tests.rs"]
mod touch_probe;
#[derive(Default)]
struct Log {
    events: Vec<(usize, f64)>,
    close_on_input: bool,
    close_on_change: bool,
}
struct Parent {
    hwnd: HWND,
    log: Box<Log>,
}
impl Parent {
    fn new() -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                300,
                200,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let mut log = Box::<Log>::default();
            assert!(
                SetWindowSubclass(hwnd, Some(record), 1, (&mut *log as *mut Log) as usize)
                    .as_bool()
            );
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Self { hwnd, log }
        }
    }
}
impl Drop for Parent {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
unsafe extern "system" fn record(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if message == NOTIFY {
        let child = HWND(lp.0 as *mut _);
        if !alive(child) {
            return LRESULT(0);
        }
        let log = &mut *(data as *mut Log);
        log.events.push((wp.0, (*state(child)).value));
        let close = (log.close_on_input && wp.0 == 0) || (log.close_on_change && wp.0 == 1);
        if close {
            let _ = DestroyWindow(child);
        }
        return LRESULT(1);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
fn range() -> Range {
    let scene =
        crate::surface_scene::Scene::parse(br#"{"id":"s","type":"slider","props":{"value":50}}"#)
            .unwrap();
    Range::parse(scene.root()).unwrap()
}
fn rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: 200,
        bottom: 30,
    }
}
fn control(parent: &Parent) -> Control {
    let control = Control::new(parent.hwnd, rect(), range()).unwrap();
    control.show().unwrap();
    unsafe {
        let _ = SetFocus(Some(control.hwnd().unwrap()));
    }
    control
}
unsafe fn key(hwnd: HWND, key: VIRTUAL_KEY) {
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(key.0 as usize)), None);
}
unsafe fn mouse(hwnd: HWND, message: u32, x: i32) {
    SendMessageW(
        hwnd,
        message,
        None,
        Some(LPARAM((x as u16 as u32 | (15 << 16)) as isize)),
    );
}
#[test]
fn surface_slider_control_keys_match_browser_input_then_commit_without_boundary_duplicates() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    for (pressed, expected) in [
        (VK_RIGHT, 51.),
        (VK_LEFT, 49.),
        (VK_UP, 51.),
        (VK_DOWN, 49.),
        (VK_HOME, 0.),
        (VK_END, 100.),
        (VK_PRIOR, 60.),
        (VK_NEXT, 40.),
    ] {
        control.set_value(50.).unwrap();
        parent.log.events.clear();
        unsafe {
            key(hwnd, pressed);
        }
        assert_eq!(control.value().unwrap(), expected);
        assert_eq!(parent.log.events, [(0, expected), (1, expected)]);
    }
    control.set_value(100.).unwrap();
    parent.log.events.clear();
    unsafe {
        key(hwnd, VK_RIGHT);
    }
    assert!(parent.log.events.is_empty());
    control.set_enabled(false).unwrap();
    unsafe {
        key(hwnd, VK_LEFT);
    }
    assert_eq!(control.value().unwrap(), 100.);
    assert!(parent.log.events.is_empty());
}
#[test]
fn surface_slider_control_drag_captures_moves_and_commits_once() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let (left, right, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        assert_eq!(GetCapture(), hwnd);
        mouse(hwnd, WM_MOUSEMOVE, right);
        mouse(hwnd, WM_MOUSEMOVE, right);
        mouse(hwnd, WM_LBUTTONUP, right);
        assert_ne!(GetCapture(), hwnd);
        mouse(hwnd, WM_LBUTTONUP, right);
    }
    assert_eq!(parent.log.events, [(0, 0.), (0, 100.), (1, 100.)]);
    assert_eq!(control.value().unwrap(), 100.);
}
#[test]
fn surface_slider_control_cancel_disable_hide_and_destroy_release_capture_without_commit() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    for mode in 0..3 {
        control.set_enabled(true).unwrap();
        control.show().unwrap();
        control.set_value(50.).unwrap();
        parent.log.events.clear();
        unsafe {
            mouse(hwnd, WM_LBUTTONDOWN, -20);
            assert_eq!(GetCapture(), hwnd);
        }
        match mode {
            0 => unsafe {
                SendMessageW(hwnd, WM_CANCELMODE, None, None);
            },
            1 => control.set_enabled(false).unwrap(),
            _ => unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            },
        }
        unsafe {
            assert_ne!(GetCapture(), hwnd);
            mouse(hwnd, WM_LBUTTONUP, 100);
        }
        assert_eq!(parent.log.events, [(0, 0.)]);
    }
    control.set_enabled(true).unwrap();
    control.show().unwrap();
    unsafe {
        mouse(hwnd, WM_LBUTTONDOWN, 100);
        assert_eq!(GetCapture(), hwnd);
    }
    drop(control);
    unsafe {
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert_ne!(GetCapture(), hwnd);
    }
}
#[test]
fn surface_slider_control_input_receiver_can_destroy_hwnd_without_late_commit() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    parent.log.close_on_input = true;
    unsafe {
        key(hwnd, VK_RIGHT);
        assert!(!IsWindow(Some(hwnd)).as_bool());
    }
    assert_eq!(parent.log.events, [(0, 51.)]);
    assert!(control.value().is_err());
}
#[test]
fn surface_slider_control_validates_owner_bounds_and_silent_source_sync() {
    use crate::desktop::surface_edit_messages as edit;
    assert!(
        ![
            edit::COMPOSITION_START,
            edit::COMPOSITION_END,
            edit::NAVIGATE,
            edit::COMMIT,
            edit::EDIT_BEGIN,
            edit::EDIT_END,
            edit::STEP_UP,
            edit::STEP_DOWN,
            edit::CAN_PASTE
        ]
        .contains(&NOTIFY),
        "Slider notification must not alias EDIT commands"
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    assert!(Control::new(HWND::default(), rect(), range()).is_err());
    assert!(Control::new(parent.hwnd, RECT { right: 0, ..rect() }, range()).is_err());
    let control = control(&parent);
    control.set_value(33.5).unwrap();
    assert_eq!(control.value().unwrap(), 34.);
    assert!(control.set_value(f64::NAN).is_err());
    assert_eq!(control.value().unwrap(), 34.);
    assert!(parent.log.events.is_empty());
    let hwnd = control.hwnd().unwrap();
    unsafe {
        RemovePropW(hwnd, OWNER).unwrap();
    }
    assert!(control.value().is_err());
    drop(control);
    unsafe {
        assert!(IsWindow(Some(hwnd)).as_bool());
    }
    drop(parent);
    unsafe {
        assert!(!IsWindow(Some(hwnd)).as_bool());
    }
}

#[test]
fn surface_slider_control_missing_state_is_rejected_but_owned_window_is_still_destroyed() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    // 覆盖初始化失败/销毁重入时 owner 仍存在但 State 已释放的边界。
    unsafe {
        take_state(hwnd);
    }
    assert!(control.value().is_err());
    assert!(control.set_value(1.).is_err());
    drop(control);
    unsafe {
        assert!(!IsWindow(Some(hwnd)).as_bool());
    }
}
