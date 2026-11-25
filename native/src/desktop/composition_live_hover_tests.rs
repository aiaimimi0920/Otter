//! Verify UI queue pause/Leave immediately revokes DLL-visible eligibility, independently of slow-worker exit receipts.
use super::*;
use otter_live_hover_hook::scope::ACTIVE;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}
fn fixture() -> (Window, Control) {
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_POPUP,
            0,
            0,
            16,
            16,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    };
    (
        Window(hwnd),
        Control {
            hover: Some(crate::live_hover_owner::Owner::new(hwnd).unwrap()),
            desired: true,
            enabled: true,
            ..Control::default()
        },
    )
}
fn active(window: &Window, control: &Control) -> bool {
    unsafe { GetPropW(window.0, ACTIVE) == control.hover.as_ref().unwrap().identity().token() }
}
#[test]
fn hover_leave_and_pause_revoke_before_worker_cleanup() {
    let (window, mut control) = fixture();
    assert!(control.push(Event::Move { point: [0.5, 0.5] }));
    assert!(active(&window, &control));
    assert!(control.push(Event::Leave));
    assert!(!active(&window, &control));
    control.push(Event::Hover { point: [0.5, 0.5] });
    assert!(active(&window, &control));
    control.pause();
    assert!(!active(&window, &control));
    assert!(control.desired);
}
#[test]
fn hover_overload_and_cancel_revoke_without_rearming_failed_event() {
    let (window, mut control) = fixture();
    control.push(Event::Move { point: [0.5, 0.5] });
    for _ in 0..queue::CAPACITY - 1 {
        assert!(control.push(Event::Character { unit: 65 }));
    }
    assert!(active(&window, &control));
    assert!(!control.push(Event::Hover { point: [0.4, 0.4] }));
    assert!(!active(&window, &control));
    assert!(control.desired);
    control.push(Event::Move { point: [0.5, 0.5] });
    control.cancel();
    assert!(!active(&window, &control));
    assert!(!control.desired);
}
