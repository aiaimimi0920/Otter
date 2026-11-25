//! Native click round-trips, soft backpressure, and host revocation for the same real child; show only owned offscreen nonactivating windows.
use super::*;
#[test]
fn timer_replies_preserve_unconsumed_animation_deadline() {
    let mut input = input::Input::new(crate::surface_javascript::fixture::binding());
    let start = std::time::Instant::now();
    input.schedule_at(start, true, Some(1), true);
    let original = start + Duration::from_millis(16);
    for tick in 1..=32 {
        let now = start + Duration::from_millis(tick);
        input.schedule_at(now, true, Some(1), true);
        assert_eq!(input.animation, Some(original));
        assert_eq!(input.timer, Some(now + Duration::from_millis(1)));
    }
    // Dispatched rAF may be rescheduled; cancellation requests and stopping must clear the schedule.
    input.animation = None;
    let next = start + Duration::from_millis(40);
    input.schedule_at(next, true, None, false);
    assert_eq!(input.animation, Some(next + Duration::from_millis(16)));
    assert_eq!(input.timer, None);
    input.schedule_at(next, false, Some(0), false);
    assert_eq!(input.animation, None);
    assert_eq!(input.timer, Some(next));
    input.clear();
    assert_eq!(input.timer, None);
}
fn shown() -> (
    Window,
    crate::device_authorization_lease::Grant,
    std::sync::mpsc::Receiver<crate::bounded_worker_process::Probe>,
) {
    let (runtime, grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let mut owner = Window::new(runtime, crate::surface_javascript::fixture::binding());
    wait(&owner, 1);
    owner.poll(|| true, false).unwrap();
    let hwnd = owner.window.as_ref().unwrap().dialog_owner().unwrap();
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::*;
        SetWindowPos(
            hwnd,
            None,
            -30000,
            -30000,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
        .unwrap();
    }
    owner.poll(|| true, true).unwrap();
    for _ in 0..3 {
        if !owner.runtime.pending() {
            break;
        }
        wait(&owner, 1);
        owner.poll(|| true, true).unwrap();
    }
    assert!(!owner.runtime.pending());
    assert!(owner.window.as_ref().unwrap().pointer_enabled());
    (owner, grant, probe)
}
#[test]
fn native_button_updates_same_signed_child_with_single_slot_backpressure() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut owner, _grant, probe) = shown();
    let window = owner.window.as_ref().unwrap();
    let button = window.javascript_first_button_for_test();
    window.javascript_queue_click_for_test(&button);
    owner.poll(|| true, true).unwrap();
    assert!(owner.runtime.pending());
    assert!(!owner.window.as_ref().unwrap().pointer_enabled());
    wait(&owner, 1);
    owner.poll(|| true, true).unwrap();
    assert_eq!(
        owner.window.as_ref().unwrap().javascript_text_for_test(),
        "1"
    );
    assert!(owner.window.as_ref().unwrap().pointer_enabled());
    owner.stop().unwrap();
    wait(&owner, 0);
    owner.poll(|| false, false).unwrap();
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
#[test]
fn host_revocation_discards_queued_native_click_without_replaying_it() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (mut owner, _grant, probe) = shown();
    let window = owner.window.as_ref().unwrap();
    window.javascript_queue_click_for_test(&window.javascript_first_button_for_test());
    owner.poll(|| false, true).unwrap();
    // poll may already have collected background terminal state and closed thread handles; do not wait on an empty handle set.
    if !owner.finished() {
        wait(&owner, 0);
    }
    owner.poll(|| false, false).unwrap();
    assert!(owner.finished());
    assert!(owner.deadline().is_none());
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
