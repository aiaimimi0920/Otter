//! The production Worker retains the original popup source; owned source-thread hooks/windows only, without WGC or external-application loading.
use super::*;
use crate::live_input::{Backend as _, Delivery};
use crate::live_input_worker::{Command, Exit, Reason, Worker};
use crate::live_window_target::{
    popup::Scope,
    recovery::popup_layer::fixture::{pump, Fixture},
};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::*,
    UI::{Controls::WM_MOUSELEAVE, WindowsAndMessaging::*},
};

thread_local! { static TRACE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) }; }
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(
        message,
        WM_KEYDOWN | WM_KEYUP | WM_MOUSEMOVE | WM_MOUSELEAVE | WM_CANCELMODE
    ) {
        TRACE.with(|trace| trace.borrow_mut().push(message));
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
fn source() -> Fixture {
    let fixture = Fixture::new();
    unsafe {
        SetWindowLongPtrW(
            fixture.popup.0,
            GWLP_WNDPROC,
            procedure as *const () as isize,
        );
    }
    fixture.show();
    TRACE.with(|trace| trace.borrow_mut().clear());
    fixture
}
fn scope(fixture: &Fixture) -> Scope {
    Scope::bind(
        &Target::bind(fixture.root.0).unwrap(),
        Some(fixture.owner.identity()),
        fixture.popup.0,
    )
    .unwrap()
}
fn worker(fixture: &Fixture) -> Worker<Error> {
    let original = scope(fixture);
    let root = original.target().unwrap();
    let transfer = original.transfer();
    Worker::start(move || {
        Backend::new_local(root, None)
            .and_then(|backend| backend.with_popup_scope(Some(transfer)))
            .map_err(|error| format!("{error:?}"))
    })
    .unwrap()
}
fn dispatch(
    worker: &mut Worker<Error>,
    command: Command,
) -> Result<Delivery, live_input::Error<Error>> {
    worker.submit(command).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        pump();
        if let Some(reply) = worker.poll_reply().unwrap() {
            return reply.result;
        }
        assert!(Instant::now() < deadline, "scoped input reply deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn finish(worker: &mut Worker<Error>) -> Exit {
    worker.stop();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        pump();
        if let Some(exit) = worker.take_exit() {
            return exit;
        }
        assert!(Instant::now() < deadline, "scoped input cleanup deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn key(sequence: u64, code: u16) -> Command {
    Command::Input {
        sequence,
        event: Event::Key { code, down: true },
    }
}

#[test]
fn revoked_epoch_rejects_new_worker_input_but_releases_the_original_held_key() {
    let fixture = source();
    let mut worker = worker(&fixture);
    dispatch(&mut worker, Command::Enable).unwrap();
    dispatch(&mut worker, key(1, 65)).unwrap();
    fixture.owner.control(false).unwrap();
    assert!(matches!(
        dispatch(&mut worker, key(2, 66)),
        Err(live_input::Error::Backend(Error::PopupScope(_)))
    ));
    let exit = finish(&mut worker);
    assert_eq!(exit.reason, Reason::InputFailed);
    assert_eq!(
        exit.cleanup,
        Some(live_input::Cleanup {
            released: 1,
            failed: 0,
            skipped: 0
        })
    );
    TRACE.with(|trace| assert_eq!(*trace.borrow(), [WM_KEYDOWN, WM_KEYUP]));
}

#[test]
fn new_display_cannot_receive_the_previous_workers_held_key_release() {
    let fixture = source();
    let mut worker = worker(&fixture);
    dispatch(&mut worker, Command::Enable).unwrap();
    dispatch(&mut worker, key(1, 65)).unwrap();
    fixture.show();
    assert!(scope(&fixture).target().is_ok());
    TRACE.with(|trace| trace.borrow_mut().clear());
    assert!(matches!(
        dispatch(&mut worker, key(2, 66)),
        Err(live_input::Error::Backend(Error::PopupScope(_)))
    ));
    let exit = finish(&mut worker);
    assert_eq!(
        exit.cleanup,
        Some(live_input::Cleanup {
            released: 0,
            failed: 0,
            skipped: 1
        })
    );
    TRACE.with(|trace| assert!(trace.borrow().is_empty()));
}

#[test]
#[ignore = "interactive Windows desktop; production coordinate mapping to owned leased HWND"]
fn old_worker_hover_cleanup_does_not_leave_the_new_display() {
    let fixture = source();
    let mut worker = worker(&fixture);
    dispatch(&mut worker, Command::Enable).unwrap();
    dispatch(
        &mut worker,
        Command::Input {
            sequence: 1,
            event: Event::Move { point: [0.5; 2] },
        },
    )
    .unwrap();
    TRACE.with(|trace| assert_eq!(*trace.borrow(), [WM_MOUSEMOVE]));
    fixture.show();
    TRACE.with(|trace| trace.borrow_mut().clear());
    assert_eq!(finish(&mut worker).cleanup.unwrap().failed, 0);
    TRACE.with(|trace| assert!(trace.borrow().is_empty()));
}

#[test]
fn send_entry_rechecks_the_original_scope_even_after_a_successful_preflight() {
    let fixture = source();
    let original = scope(&fixture);
    let root = original.target().unwrap();
    let target = Destination::bind(&root, root.hwnd()).unwrap();
    let mut backend = Backend::new_local(root, None)
        .unwrap()
        .with_popup_scope(Some(original.transfer()))
        .unwrap();
    backend.preflight().unwrap();
    fixture.show();
    TRACE.with(|trace| trace.borrow_mut().clear());
    assert!(matches!(
        backend.send(
            target,
            Event::Key {
                code: 65,
                down: true
            },
            0,
            250
        ),
        Err(Error::PopupScope(_))
    ));
    TRACE.with(|trace| assert!(trace.borrow().is_empty()));
}

#[test]
fn popup_transfer_is_rejected_for_remote_or_a_different_backend_root() {
    let fixture = source();
    let popup = scope(&fixture);
    assert!(matches!(
        Backend::new(popup.target().unwrap(), None)
            .unwrap()
            .with_popup_scope(Some(popup.transfer())),
        Err(Error::InvalidDelivery)
    ));
    assert!(matches!(
        Backend::new_local(Target::bind(fixture.root.0).unwrap(), None)
            .unwrap()
            .with_popup_scope(Some(popup.transfer())),
        Err(Error::PopupScope(_))
    ));
}
