//! Session control-cycle handoff: real Worker, bounded fake capture, and owned source windows, without WGC or the product DLL.
use super::super::{LiveJob, LiveWindow, Texture};
use super::*;
#[path = "composition_live_input_idle_tests.rs"]
mod idle;
use crate::{
    desktop::Apartment,
    live_input::{Backend as InputBackend, Cleanup},
    live_window_target::{
        input_backend::Error,
        input_message,
        recovery::popup_layer::fixture::{pump, Fixture},
        Target,
    },
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

fn session(source: &Fixture) -> Session {
    let active = Arc::new(AtomicUsize::new(0));
    let (job, window) = LiveJob::<Texture>::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, 80, 60).map_err(io::Error::other),
        |_, cancel| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while !cancel.load(Ordering::Acquire) {
                if Instant::now() >= deadline {
                    return Err("fake capture was not stopped".into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        },
    )
    .unwrap();
    window.allow_input().unwrap();
    window.request_default_input();
    let mut session = Session::new(job, window);
    session.target = Some(Target::bind(source.root.0).unwrap());
    session
}
fn settle(session: &mut Session, ready: impl Fn(&Session) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        pump();
        session.poll_input();
        if ready(session) {
            return;
        }
        assert!(Instant::now() < deadline, "input cycle did not settle");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn finish(session: &mut Session) -> Result<(), String> {
    session.stop();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        pump();
        if let Some(result) = session.poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "session cleanup did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn title(session: &Session) -> String {
    let mut text = [0; 128];
    let count = unsafe { GetWindowTextW(session.window.as_ref().unwrap().popup_hwnd(), &mut text) };
    String::from_utf16(&text[..count as usize]).unwrap()
}

#[test]
fn default_enable_publishes_the_initial_cycle_before_starting_the_worker() {
    let _apartment = Apartment::new().unwrap();
    let source = Fixture::new();
    let mut session = session(&source);
    let identity = session.window.as_ref().unwrap().hover_identity().unwrap();
    assert!(
        identity.capture_control().is_err(),
        "default intent starts at epoch zero"
    );
    session.process_window_input();
    let admitted = identity.capture_control().unwrap();
    settle(&mut session, |session| session.input_enabled());
    session.process_window_input();
    assert!(admitted.valid());
    assert!(session.input_error.is_none());
    assert!(!session.window.as_ref().unwrap().take_input_notice());
    assert_eq!(
        title(&session),
        crate::ui_text::text("integration-live-controlling")
    );
    finish(&mut session).unwrap();
}

#[test]
fn late_old_cycle_input_retires_without_cancelling_the_new_ui_enable_intent() {
    let _apartment = Apartment::new().unwrap();
    let source = Fixture::new();
    let mut session = session(&source);
    session.process_window_input();
    settle(&mut session, |session| session.input_enabled());
    let window = session.window.as_ref().unwrap();
    let identity = window.hover_identity().unwrap();
    let old = identity.capture_control().unwrap();
    // Keep the old worker alive to deterministically model unsettled revocation notifications/replies.
    window.input_status(true, false, None);
    window.suspend_source_input();
    window.request_default_input();
    window.input_status(false, true, None);
    let current = identity.capture_control().unwrap();
    assert!(!old.valid());
    assert_ne!(old.epoch(), current.epoch());
    session
        .send_input(
            1,
            Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    settle(&mut session, |session| session.input.is_none());
    assert!(session.input_error.is_none());
    assert_eq!(session.input_cleanup, Some(Cleanup::default()));
    assert!(session.window.as_ref().unwrap().input_resume_pending());
    assert!(!session.window.as_ref().unwrap().take_input_notice());
    assert!(current.valid());
    session.process_window_input();
    settle(&mut session, |session| session.input_enabled());
    session.process_window_input();
    assert!(current.valid());
    assert_eq!(
        session.last_input_sequence, 1,
        "old input must not be replayed"
    );
    assert_eq!(
        title(&session),
        crate::ui_text::text("integration-live-controlling")
    );
    finish(&mut session).unwrap();
}

#[test]
fn snapshot_failure_rolls_back_pending_state_without_creating_a_worker() {
    let _apartment = Apartment::new().unwrap();
    let source = Fixture::new();
    let mut session = session(&source);
    let window = session.window.as_ref().unwrap();
    let old = window.hover_identity().unwrap();
    let replacement = crate::live_hover_owner::Owner::new(window.popup_hwnd()).unwrap();
    assert!(!old.valid());
    session.process_window_input();
    assert!(session.input.is_none());
    assert!(session.input_error.is_some());
    assert!(!session.input_enabled());
    assert_eq!(
        session.window.as_ref().unwrap().take_input_request(),
        Some(false)
    );
    assert!(replacement.identity().capture_control().is_err());
    assert!(session.window.as_ref().unwrap().take_input_notice());
    assert_eq!(
        title(&session),
        crate::ui_text::text("integration-live-control-unavailable")
    );
    finish(&mut session).unwrap();
}

struct Reject {
    error: Option<Error>,
    cleanup_failed: bool,
}
impl InputBackend for Reject {
    type Target = u8;
    type Error = Error;
    fn preflight(&mut self) -> Result<(), Error> {
        Err(self.error.take().expect("only one enable attempt"))
    }
    fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Error> {
        unreachable!()
    }
    fn send(&mut self, _: u8, _: Event, _: u8, _: u32) -> Result<(), Error> {
        unreachable!()
    }
    fn cleanup_target(&mut self, _: u8) -> Option<u8> {
        None
    }
    fn release(&mut self, _: u8, _: Event, _: u8) -> Result<(), Error> {
        unreachable!()
    }
    fn leave(&mut self) -> Cleanup {
        Cleanup {
            failed: u8::from(self.cleanup_failed),
            ..Default::default()
        }
    }
}

#[test]
fn committed_delivery_failure_or_failed_cleanup_still_reports_unavailable() {
    let _apartment = Apartment::new().unwrap();
    for (error, cleanup_failed) in [
        (Error::Message(input_message::Error::DeliveryFailed), false),
        (Error::ControlRevoked, true),
    ] {
        let source = Fixture::new();
        let mut session = session(&source);
        let window = session.window.as_ref().unwrap();
        window.input_status(false, true, None);
        let identity = window.hover_identity().unwrap();
        let old = identity.capture_control().unwrap();
        window.suspend_source_input();
        window.request_default_input();
        window.input_status(false, true, None);
        assert!(!old.valid());
        let mut worker = Worker::start_for_ui(move || {
            Ok(Reject {
                error: Some(error),
                cleanup_failed,
            })
        })
        .unwrap();
        worker.submit(Command::Enable).unwrap();
        session.input = Some(worker);
        settle(&mut session, |session| session.input.is_none());
        assert!(session.input_error.is_some());
        assert!(session.window.as_ref().unwrap().take_input_notice());
        assert!(!session.window.as_ref().unwrap().input_resume_pending());
        assert_eq!(
            title(&session),
            crate::ui_text::text("integration-live-control-unavailable")
        );
        let result = finish(&mut session);
        assert_eq!(result.is_err(), cleanup_failed);
    }
}
