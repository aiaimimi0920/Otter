//! 即时交付只操作自有隐藏窗口及合成图；不捕获桌面、不发送系统输入。
use super::*;
use std::time::{Duration, Instant};

unsafe fn window() -> crate::desktop::Window {
    crate::desktop::Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("capture ready fixture"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}
fn image() -> super::super::super::image::Image {
    super::super::super::image::Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    }
}
fn install(state: &State, wait_for_cancel: bool) -> Arc<AtomicBool> {
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        while wait_for_cancel && !signal.load(Ordering::Acquire) {
            assert!(Instant::now() < deadline, "worker was not cancelled");
            std::thread::yield_now();
        }
        PreparedDocument::from_image(image(), true)
            .map(|document| Some(vec![document]))
            .map_err(|error| error.to_string())
    });
    *state.open_job.borrow_mut() = Some(OpenJob {
        new_pin: true,
        worker: OpenWorker {
            handle: Some(worker),
            cancel: cancel.clone(),
            _permit: crate::edit_worker_budget::Permit::acquire_open().unwrap(),
        },
        token: DocumentToken::capture(&state.canvas.borrow()),
    });
    cancel
}
#[test]
fn completed_worker_is_delivered_without_any_timer() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        install(&state, false);
        finish(window.0, &state).unwrap();
        assert!(state.open_job.borrow().is_none());
        assert_eq!(state.new_pin.borrow().as_ref().unwrap()[0].image.width, 1);
    }
}
#[test]
fn queued_escape_cancels_and_joins_instead_of_publishing() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        let cancel = install(&state, true);
        PostMessageW(Some(window.0), WM_KEYDOWN, WPARAM(0x1b), LPARAM(0)).unwrap();
        finish(window.0, &state).unwrap();
        assert!(cancel.load(Ordering::Acquire));
        assert!(state.open_job.borrow().is_none());
        assert!(state.new_pin.borrow().is_none());
    }
}
#[test]
fn queued_quit_preserves_exit_code_and_leaves_decoder_for_shutdown() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        let (release, gate) = std::sync::mpsc::channel();
        let cancel = install_gated(&state, true, gate);
        PostQuitMessage(73);
        finish(window.0, &state).unwrap();
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
        assert_eq!(message.wParam.0, 73);
        assert!(cancel.load(Ordering::Acquire));
        let worker = retire(&state).unwrap();
        assert!(!worker.is_finished());
        assert!(state.new_pin.borrow().is_none());
        release.send(()).unwrap();
        drop(worker);
    }
}
#[test]
fn destroyed_owner_returns_while_decoder_is_still_blocked() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        let (release, gate) = std::sync::mpsc::channel();
        let cancel = install_gated(&state, true, gate);
        DestroyWindow(window.0).unwrap();
        finish(window.0, &state).unwrap();
        assert!(cancel.load(Ordering::Acquire));
        let worker = retire(&state).unwrap();
        assert!(!worker.is_finished());
        assert!(state.new_pin.borrow().is_none());
        release.send(()).unwrap();
        drop(worker);
    }
}
#[test]
fn ready_result_is_not_starved_by_queued_messages() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        install(&state, false);
        let deadline = Instant::now() + Duration::from_secs(3);
        while !state
            .open_job
            .borrow()
            .as_ref()
            .unwrap()
            .worker
            .is_finished()
        {
            assert!(Instant::now() < deadline, "worker did not finish");
            std::thread::yield_now();
        }
        let marker = WM_APP + 0x6a;
        for _ in 0..32 {
            PostMessageW(Some(window.0), marker, WPARAM(0), LPARAM(0)).unwrap();
        }
        finish(window.0, &state).unwrap();
        let mut message = MSG::default();
        let mut remaining = 0;
        while PeekMessageW(&mut message, Some(window.0), marker, marker, PM_REMOVE).as_bool() {
            remaining += 1;
        }
        assert!(
            remaining > 0,
            "completion must not wait for an empty message queue"
        );
        assert!(state.new_pin.borrow().is_some());
        assert!(state.open_job.borrow().is_none());
    }
}
#[test]
fn no_pending_worker_returns_without_creating_a_task() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        finish(window.0, &state).unwrap();
        assert!(state.open_job.borrow().is_none());
        assert!(state.new_pin.borrow().is_none());
    }
}
#[test]
fn capture_start_does_not_take_over_an_existing_file_job() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        let cancel = install(&state, true);
        let error = super::super::start_capture_destination(window.0, &state, true, || {
            panic!("Cache publication must not run while an image job is active")
        })
        .unwrap_err();
        assert_eq!(error.code(), E_ABORT);
        assert!(!cancel.load(Ordering::Acquire));
        assert!(state.open_job.borrow().is_some());
        assert!(state.new_pin.borrow().is_none());
        drop(retire(&state));
        assert!(cancel.load(Ordering::Acquire));
        assert!(state.open_job.borrow().is_none());
    }
}

#[test]
fn capture_does_not_publish_while_another_pin_is_awaiting_creation() {
    unsafe {
        let state = State::new(None, true).unwrap();
        let window = window();
        install(&state, false);
        finish(window.0, &state).unwrap();
        let error = super::super::start_capture_destination(window.0, &state, true, || {
            panic!("Cache publication must not run while a prepared pin is queued")
        })
        .unwrap_err();
        assert_eq!(error.code(), E_UNEXPECTED);
        assert!(state.open_job.borrow().is_none());
        assert_eq!(state.new_pin.borrow().as_ref().unwrap().len(), 1);
    }
}
