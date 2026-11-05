//! 用受控 worker 阻塞点验证非阻塞取消，不以机器速度/毫秒阈值断言代替所有权验证。
use super::*;
use std::sync::mpsc;

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
        SetCapture(fixture.hwnd);
        dispatch(fixture.hwnd, &state, WM_LBUTTONUP, WPARAM(0), LPARAM(0));
        assert!(signal.load(Ordering::Acquire));
        assert!(
            !timed_out.load(Ordering::Acquire),
            "cancel waited for worker completion"
        );
        assert!(state.drag.pending.borrow().is_some());
        assert!(state.drag.origin.get().is_none());
        assert_ne!(GetCapture(), fixture.hwnd);
        assert!(dispatch(fixture.hwnd, &state, WM_MOUSEMOVE, WPARAM(0), LPARAM(0)).is_none());
        assert!(start(fixture.hwnd, &state, Ok(fixture.root.clone())).is_err());
        // 新编辑手势接管捕获后，旧任务即使被回收也不能抢走它。
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
        assert_eq!(GetCapture(), fixture.hwnd);
        ReleaseCapture().unwrap();
        assert!(!fixture.root.exists());
    }
}

#[test]
fn destroy_cancels_and_joins_before_losing_window_owner() {
    unsafe {
        let fixture = Fixture::new();
        let state = state();
        let signal = Arc::new(AtomicBool::new(false));
        let worker_signal = signal.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let worker_exited = exited.clone();
        let worker = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !worker_signal.load(Ordering::Acquire) && Instant::now() < deadline {
                std::thread::yield_now();
            }
            worker_exited.store(true, Ordering::Release);
            Ok(None)
        });
        let stamp = crate::desktop::display_snapshot::Stamp::capture(&state.canvas.borrow(), None);
        *state.drag.pending.borrow_mut() = Some(Pending {
            job: drag_prepare::Job::from_test_worker(worker, signal.clone()),
            stamp,
        });
        dispatch(fixture.hwnd, &state, WM_DESTROY, WPARAM(0), LPARAM(0));
        assert!(signal.load(Ordering::Acquire));
        assert!(exited.load(Ordering::Acquire));
        assert!(state.drag.pending.borrow().is_none());
        assert!(!fixture.root.exists());
    }
}
