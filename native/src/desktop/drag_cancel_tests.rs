//! Verify nonblocking cancellation using controlled worker barriers; ownership evidence, not machine-speed/millisecond thresholds.
use super::*;
use std::sync::mpsc;

#[test]
fn uncommitted_gesture_and_active_ole_each_hold_the_busy_gate() {
    let drag = Session::default();
    assert!(!drag.is_busy());
    drag.origin.set(Some((1, 1)));
    assert!(drag.is_busy());
    drag.origin.set(None);
    *drag.active.borrow_mut() = Some(Arc::new(AtomicBool::new(false)));
    assert!(drag.is_busy());
    drag.active.borrow_mut().take();
    assert!(!drag.is_busy());
}

#[test]
fn cancelled_worker_keeps_single_owner_without_blocking_input_or_releasing_new_capture() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        let signal = Arc::new(AtomicBool::new(false));
        let (release, wait_release) = mpsc::channel();
        let timed_out = Arc::new(AtomicBool::new(false));
        let observed = timed_out.clone();
        let worker = std::thread::spawn(move || {
            if wait_release.recv_timeout(Duration::from_secs(5)).is_err() {
                observed.store(true, Ordering::Release);
            }
            Ok(None)
        });
        let stamp = crate::desktop::display_snapshot::Stamp::capture(&state.canvas.borrow(), None);
        *state.drag.pending.borrow_mut() = Some(Pending {
            job: drag_prepare::Job::from_test_worker(worker, signal.clone()),
            stamp,
        });
        state.drag.origin.set(Some((1, 1)));
        assert!(state.drag.is_busy());
        SetCapture(fixture.hwnd);
        dispatch(fixture.hwnd, &state, WM_LBUTTONUP, WPARAM(0), LPARAM(0));
        assert!(signal.load(Ordering::Acquire));
        assert!(
            !timed_out.load(Ordering::Acquire),
            "cancel waited for worker completion"
        );
        assert!(state.drag.pending.borrow().is_some());
        assert!(state.drag.origin.get().is_none());
        assert!(
            state.drag.is_busy(),
            "cancel does not release the worker slot"
        );
        assert_ne!(GetCapture(), fixture.hwnd);
        assert!(dispatch(fixture.hwnd, &state, WM_MOUSEMOVE, WPARAM(0), LPARAM(0)).is_none());
        assert!(start(fixture.hwnd, &state, Ok(fixture.root.clone())).is_err());
        // After a new editing gesture takes capture, old task reclamation must not steal it.
        SetCapture(fixture.hwnd);
        finish::poll_using(fixture.hwnd, &state, true, |_, _| {
            panic!("cancelled drag published")
        });
        assert_eq!(GetCapture(), fixture.hwnd);
        release.send(()).unwrap();
        wait(&state);
        finish::poll_using(fixture.hwnd, &state, true, |_, _| {
            panic!("cancelled drag published")
        });
        assert!(state.drag.pending.borrow().is_none());
        assert!(!state.drag.is_busy());
        assert_eq!(GetCapture(), fixture.hwnd);
        ReleaseCapture().unwrap();
        assert!(!fixture.root.exists());
    }
}

#[test]
fn destroy_cancels_but_leaves_unfinished_worker_for_owner_transfer() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        let signal = Arc::new(AtomicBool::new(false));
        let (release, wait_release) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _ = wait_release.recv_timeout(Duration::from_secs(5));
            Ok(None)
        });
        let stamp = crate::desktop::display_snapshot::Stamp::capture(&state.canvas.borrow(), None);
        *state.drag.pending.borrow_mut() = Some(Pending {
            job: drag_prepare::Job::from_test_worker(worker, signal.clone()),
            stamp,
        });
        dispatch(fixture.hwnd, &state, WM_DESTROY, WPARAM(0), LPARAM(0));
        assert!(signal.load(Ordering::Acquire));
        let retired = state.drag.retire_preparation().unwrap();
        assert!(!retired.is_finished());
        assert!(state.drag.pending.borrow().is_none());
        release.send(()).unwrap();
        drop(retired);
        assert!(!fixture.root.exists());
    }
}
