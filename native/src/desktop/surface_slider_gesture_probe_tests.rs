//! 单指轴向配置与真实 WM_GESTURE 默认转发探针，不宣称父滚动已实现。
use super::*;
use windows::Win32::{System::SystemServices::*, UI::Input::Touch::*};
#[path = "surface_slider_gesture_lifecycle_probe_tests.rs"]
mod lifecycle;

#[test]
fn surface_slider_gesture_config_requests_only_vertical_single_finger_pan() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let mut count = 1;
    let mut config = GESTURECONFIG {
        dwID: GID_PAN,
        ..Default::default()
    };
    unsafe {
        GetGestureConfig(
            control.hwnd().unwrap(),
            0,
            0,
            &raw mut count,
            &mut config,
            std::mem::size_of::<GESTURECONFIG>() as u32,
        )
        .unwrap()
    };
    assert_ne!(config.dwWant & GC_PAN_WITH_SINGLE_FINGER_VERTICALLY.0, 0);
    assert_ne!(config.dwWant & GC_PAN_WITH_INERTIA.0, 0);
    assert_ne!(config.dwBlock & GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY.0, 0);
    assert_eq!(config.dwWant & GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY.0, 0);
}

#[test]
#[ignore = "explicit interactive desktop gesture routing probe"]
fn surface_slider_system_touch_probe_vertical_gesture_routing() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TOUCH_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    unsafe { InitializeTouchInjection(1, TOUCH_FEEDBACK_NONE).unwrap() };
    for horizontal in [false, true] {
        let parent = unsafe { parent() };
        let control = control(&parent);
        let hwnd = control.hwnd().unwrap();
        unsafe {
            SetGestureConfig(
                parent.hwnd,
                0,
                &[GESTURECONFIG {
                    dwID: GID_PAN,
                    dwWant: (GC_PAN
                        | GC_PAN_WITH_SINGLE_FINGER_VERTICALLY
                        | GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY
                        | GC_PAN_WITH_INERTIA)
                        .0,
                    dwBlock: GC_PAN_WITH_GUTTER.0,
                }],
                std::mem::size_of::<GESTURECONFIG>() as u32,
            )
            .unwrap();
            SetGestureConfig(
                hwnd,
                0,
                &[GESTURECONFIG {
                    dwID: GID_PAN,
                    dwWant: (GC_PAN | GC_PAN_WITH_SINGLE_FINGER_VERTICALLY | GC_PAN_WITH_INERTIA).0,
                    dwBlock: (GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY | GC_PAN_WITH_GUTTER).0,
                }],
                std::mem::size_of::<GESTURECONFIG>() as u32,
            )
            .unwrap();
            let _ = SetForegroundWindow(parent.hwnd);
            SetWindowPos(
                parent.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            )
            .unwrap();
        }
        let observer = unsafe { Observer::new(hwnd) };
        let parent_observer = unsafe { Observer::new(parent.hwnd) };
        let mut start = POINT { x: 100, y: 15 };
        unsafe { assert!(ClientToScreen(hwnd, &mut start).as_bool()) };
        pump(100);
        let mut contact = unsafe { Contact::new(hwnd, start) };
        unsafe {
            contact.send(
                start,
                POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
            );
            let mut end = start;
            for step in 1..=5 {
                end = POINT {
                    x: start.x + if horizontal { step * 12 } else { 0 },
                    y: start.y + if horizontal { 0 } else { step * 12 },
                };
                contact.send(
                    end,
                    POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
                );
            }
            contact.send(end, POINTER_FLAG_UP);
        }
        pump(600);
        println!(
            "GESTURE_PROBE horizontal={horizontal}: value={} events={:?} child={:?} parent={:?}",
            control.value().unwrap(),
            parent.log.events,
            observer.log,
            parent_observer.log
        );
        assert!(!observer.log.overflow && !parent_observer.log.overflow);
        assert!(observer.log.pointers > 0);
    }
}

#[test]
#[ignore = "explicit system touch into actual scrolling Surface"]
fn surface_slider_system_touch_probe_real_surface_scroll() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TOUCH_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    unsafe { InitializeTouchInjection(1, TOUCH_FEEDBACK_NONE).unwrap() };
    for horizontal in [false, true] {
        crate::desktop::surface_window::scroll_tests::slider_scroll_probe_fixture::run(
            horizontal,
            |parent, child| unsafe {
                let _ = SetForegroundWindow(parent);
                SetWindowPos(
                    parent,
                    Some(HWND_TOPMOST),
                    400,
                    300,
                    0,
                    0,
                    SWP_NOSIZE | SWP_SHOWWINDOW,
                )
                .unwrap();
                let observer = Observer::new(child);
                let parent_observer = Observer::new(parent);
                let mut bounds = RECT::default();
                GetClientRect(child, &mut bounds).unwrap();
                let mut start = POINT {
                    x: (bounds.right - bounds.left) / 2,
                    y: (bounds.bottom - bounds.top) / 2,
                };
                assert!(ClientToScreen(child, &mut start).as_bool());
                pump(100);
                let mut contact = Contact::new(child, start);
                contact.send(
                    start,
                    POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
                );
                let mut end = start;
                for step in 1..=5 {
                    end = POINT {
                        x: start.x + if horizontal { step * 12 } else { 0 },
                        y: start.y - if horizontal { 0 } else { step * 12 },
                    };
                    contact.send(
                        end,
                        POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
                    );
                }
                contact.send(end, POINTER_FLAG_UP);
                pump(800);
                println!(
                    "SURFACE_MESSAGES horizontal={horizontal} child={:?} parent={:?}",
                    observer.log, parent_observer.log
                );
                assert!(!observer.log.overflow);
                assert!(!parent_observer.log.overflow);
                assert!(observer.log.pointers > 0);
                assert_ne!(GetCapture(), child);
            },
        );
    }
}
