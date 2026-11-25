//! Reuse Session/real Worker; asynchronous errors without subsequent input must not be treated as normal cycle retirement.
use super::*;
use std::sync::atomic::AtomicBool;

struct AsyncReject {
    rejected: Arc<AtomicBool>,
    pending: bool,
    cleanup_failed: bool,
    released: Arc<AtomicUsize>,
}
impl InputBackend for AsyncReject {
    type Target = u8;
    type Error = Error;
    fn idle(&mut self) -> Result<(), Error> {
        if self.pending && self.rejected.load(Ordering::Acquire) {
            Err(Error::Message(input_message::Error::DeliveryFailed))
        } else {
            Ok(())
        }
    }
    fn pending_delivery(&mut self) -> bool {
        self.pending
    }
    fn delivery(&mut self) -> crate::live_input::Delivery {
        if self.pending {
            crate::live_input::Delivery::Pending
        } else {
            crate::live_input::Delivery::Completed
        }
    }
    fn preflight(&mut self) -> Result<(), Error> {
        Ok(())
    }
    fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Error> {
        Ok(1)
    }
    fn send(&mut self, _: u8, _: Event, _: u8, _: u32) -> Result<(), Error> {
        self.pending = true;
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        Some(target)
    }
    fn release(&mut self, _: u8, _: Event, _: u8) -> Result<(), Error> {
        self.released.fetch_add(1, Ordering::AcqRel);
        if self.cleanup_failed {
            Err(Error::InvalidDelivery)
        } else {
            Ok(())
        }
    }
    fn leave(&mut self) -> Cleanup {
        self.pending = false;
        Cleanup::default()
    }
}

#[test]
fn asynchronous_rejection_marks_session_unavailable_without_new_input_or_replay() {
    let _apartment = Apartment::new().unwrap();
    for cleanup_failed in [false, true] {
        let source = Fixture::new();
        let mut session = session(&source);
        session
            .window
            .as_ref()
            .unwrap()
            .input_status(false, true, None);
        let rejected = Arc::new(AtomicBool::new(false));
        let released = Arc::new(AtomicUsize::new(0));
        let backend = AsyncReject {
            rejected: rejected.clone(),
            released: released.clone(),
            pending: false,
            cleanup_failed,
        };
        let mut worker = Worker::start_for_ui(move || Ok(backend)).unwrap();
        worker.submit(Command::Enable).unwrap();
        session.input = Some(worker);
        settle(&mut session, |session| session.input_enabled());
        session
            .window
            .as_ref()
            .unwrap()
            .input_status(true, false, None);
        session
            .send_input(
                1,
                Event::Key {
                    code: 65,
                    down: true,
                },
            )
            .unwrap();
        settle(&mut session, |session| {
            session.input.as_ref().unwrap().outstanding() == 0
        });
        // A real rejection while the original cycle is revoked and the new one awaits recovery must not be swallowed as ControlRevoked.
        let window = session.window.as_ref().unwrap();
        window.suspend_source_input();
        window.request_default_input();
        window.input_status(false, true, None);
        rejected.store(true, Ordering::Release);
        settle(&mut session, |session| session.input.is_none());
        assert!(!session.input_enabled());
        assert!(session
            .input_error
            .as_ref()
            .unwrap()
            .contains("DeliveryFailed"));
        assert_eq!(session.last_input_sequence, 1);
        assert_eq!(released.load(Ordering::Acquire), 1);
        let cleanup = session.input_cleanup.unwrap();
        assert_eq!(cleanup.failed, u8::from(cleanup_failed));
        assert_eq!(cleanup.released, u8::from(!cleanup_failed));
        assert!(session.window.as_ref().unwrap().take_input_notice());
        assert!(!session.window.as_ref().unwrap().input_resume_pending());
        assert_eq!(title(&session), "Otter Live · 控制不可用");
        assert_eq!(finish(&mut session).is_err(), cleanup_failed);
    }
}
