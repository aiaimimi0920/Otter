//! 已开始的真实单指滚动在撤权/换场景后不得被后续输入或惯性复活。
use super::*;

#[test]
#[ignore = "explicit system touch lifecycle test on owned Surface"]
fn surface_slider_system_touch_probe_lifecycle_cutoff() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TOUCH_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    let old = unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    assert!(!old.0.is_null());
    let _dpi = Dpi(old);
    unsafe { InitializeTouchInjection(1, TOUCH_FEEDBACK_NONE).unwrap() };
    for replace in [false, true] {
        crate::desktop::surface_window::scroll_tests::slider_scroll_probe_fixture::cutoff(
            replace,
            |parent, child, cutoff| unsafe {
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
                for step in 1..=8 {
                    // 后续触点保持在自有父窗口范围；不在销毁整个窗口后继续注入。
                    end = POINT {
                        x: start.x,
                        y: start.y - step * 12,
                    };
                    contact.send(
                        end,
                        POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT,
                    );
                    if step == 4 {
                        println!(
                            "CUTOFF_ATTEMPT replace={replace} parent={:?}",
                            parent_observer.log
                        );
                        cutoff();
                    }
                }
                contact.send(end, POINTER_FLAG_UP);
                pump(800);
                println!(
                    "CUTOFF_MESSAGES replace={replace} parent={:?}",
                    parent_observer.log
                );
                assert!(!parent_observer.log.overflow);
                assert!(
                    parent_observer
                        .log
                        .gestures
                        .iter()
                        .any(|(id, flags, _)| *id == GID_PAN.0 && flags & GF_INERTIA != 0),
                    "system inertia was not exercised"
                );
                assert!(
                    parent_observer
                        .log
                        .gestures
                        .iter()
                        .any(|(id, _, _)| *id == GID_END.0),
                    "gesture sequence did not end"
                );
                assert_ne!(GetCapture(), child);
            },
        );
    }
}
