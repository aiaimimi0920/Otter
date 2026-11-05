//! 只用自有屏外窗口验证前台guard的清理边界；不发送全局输入或操作外部进程。
use super::*;
use crate::surface_javascript::fixture::{focus_foreground, SERIAL};
use windows::Win32::Foundation::HWND;

// 反例即使panic也从自有窗口切回anchor；用户已切到外部窗口时同样不覆盖。
struct Anchor(HWND);
impl Drop for Anchor {
    fn drop(&mut self) {
        unsafe {
            let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
            if thread == windows::Win32::System::Threading::GetCurrentThreadId() {
                let _ = SetForegroundWindow(self.0);
            }
        }
    }
}
fn window() -> SurfaceWindow {
    SurfaceWindow::new(
        crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column"}"#).unwrap(),
        120,
        80,
    )
    .unwrap()
}

#[test]
fn javascript_foreground_guard_does_not_override_a_later_foreground_choice() {
    let _serial = SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let anchor = window();
    let target = window();
    let choice = window();
    let _original = unsafe { focus_foreground(anchor.hwnd) };
    let _restore_anchor = Anchor(anchor.hwnd);
    let target_guard = unsafe { focus_foreground(target.hwnd) };
    let choice_guard = unsafe { focus_foreground(choice.hwnd) };
    assert_eq!(unsafe { GetForegroundWindow() }, choice.hwnd);
    drop(target_guard);
    assert_eq!(
        unsafe { GetForegroundWindow() },
        choice.hwnd,
        "dropping an inactive foreground guard must preserve the later choice"
    );
    drop(choice_guard);
    assert_eq!(unsafe { GetForegroundWindow() }, target.hwnd);
    assert!(
        anchor.take_error().is_none()
            && target.take_error().is_none()
            && choice.take_error().is_none()
    );
}

#[test]
fn javascript_foreground_guard_restores_owned_foreground_on_unwind() {
    let _serial = SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let anchor = window();
    let target = window();
    let _original = unsafe { focus_foreground(anchor.hwnd) };
    let _restore_anchor = Anchor(anchor.hwnd);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _target_guard = unsafe { focus_foreground(target.hwnd) };
        panic!("expected foreground unwind probe");
    }));
    assert_eq!(
        outcome.unwrap_err().downcast_ref::<&str>(),
        Some(&"expected foreground unwind probe")
    );
    assert_eq!(unsafe { GetForegroundWindow() }, anchor.hwnd);
    assert!(anchor.take_error().is_none() && target.take_error().is_none());
}
