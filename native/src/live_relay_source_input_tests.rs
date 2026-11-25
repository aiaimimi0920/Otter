//! Single-in-flight bridge acknowledges only actual execution replies; stop bypasses queued waits without requiring UI to consume another request.
use super::*;
use std::time::{Duration, Instant};
fn take(bridge: &Bridge) -> Request {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(request) = bridge.take() {
            return request;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn source_input_bridge_waits_for_actual_reply_and_rejects_second_inflight() {
    let bridge = Bridge::new();
    let worker = bridge.clone();
    let thread = std::thread::spawn(move || worker.dispatch(Action::Enable, || true));
    let request = take(&bridge);
    assert!(!thread.is_finished());
    assert_eq!(bridge.dispatch(Action::Release, || true), Err(Error::Busy));
    request.finish(Ok(()));
    assert_eq!(thread.join().unwrap(), Ok(()));
    bridge.cancel();
    assert_eq!(
        bridge.dispatch(Action::Enable, || true),
        Err(Error::Stopped)
    );
}
#[test]
fn source_input_bridge_cancel_wakes_ui_only_once() {
    let bridge = Bridge::new();
    // Simulate network-thread cancellation first, then UI handling the wake and continuing to await input-worker release.
    let worker = bridge.clone();
    std::thread::spawn(move || worker.cancel()).join().unwrap();
    let mut message = MSG::default();
    assert!(unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool());
    assert_eq!(message.message, WM_NULL);
    for _ in 0..8 {
        bridge.cancel();
    }
    assert!(!unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool());
    assert!(bridge.stopping());
}

#[test]
fn source_input_bridge_cancel_discards_late_request_without_ack_or_requeue() {
    let bridge = Bridge::new();
    let worker = bridge.clone();
    let thread = std::thread::spawn(move || {
        worker.dispatch(
            Action::Input(Event::Key {
                code: 65,
                down: true,
            }),
            || true,
        )
    });
    let request = take(&bridge);
    bridge.cancel();
    assert_eq!(thread.join().unwrap(), Err(Error::Stopped));
    request.finish(Ok(()));
    assert!(bridge.take().is_none());
}
