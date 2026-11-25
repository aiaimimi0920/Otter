//! Attached-input failures survive member retirement while ordinary attached-capture errors stay isolated; owned fake capture/input backends only.
use super::*;

#[derive(Clone, Copy)]
enum Release {
    Complete,
    Failed,
    Unknown,
}
impl crate::live_input::Backend for Release {
    type Target = u8;
    type Error = crate::live_window_target::input_backend::Error;
    fn preflight(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Self::Error> {
        Ok(1)
    }
    fn send(&mut self, _: u8, _: Event, _: u8, _: u32) -> Result<(), Self::Error> {
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        Some(target)
    }
    fn release(&mut self, _: u8, _: Event, _: u8) -> Result<(), Self::Error> {
        match self {
            Self::Complete => Ok(()),
            Self::Failed => Err(Self::Error::InvalidDelivery),
            Self::Unknown => panic!("fixture release outcome unknown"),
        }
    }
}
fn input(child: &mut Session, outcome: Release) {
    let mut worker = Worker::start_for_ui(move || Ok(outcome)).unwrap();
    worker.submit(Command::Enable).unwrap();
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while worker.outstanding() != 0 {
        if let Some(reply) = worker.poll_reply().unwrap() {
            reply.result.unwrap();
        } else {
            assert!(Instant::now() < deadline);
            unsafe {
                WaitForSingleObject(HANDLE(worker.wake_handle().as_raw_handle()), 100);
            }
        }
    }
    child.input = Some(worker);
    child.input_enabled = true;
}
fn append(parent: &mut Session, child: Session) {
    parent.popups.members.push(Member {
        hwnd: 0,
        placement: rect(0, 0, 100, 100),
        session: child,
    });
}
fn reap_children(root: &mut Session) {
    root.popups.stop();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !root.popups.is_empty() {
        assert!(root.poll().is_none(), "parent must remain running");
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(root.window.is_some());
    assert!(!root.stopping);
    assert!(root.error.is_none());
    assert!(root.input_error.is_none());
}
fn finish(root: &mut Session) -> Result<(), String> {
    root.stop();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = root.poll() {
            return result;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn retired_child_cleanup_is_reported_on_parent_stop_without_closing_parent_live() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    for outcome in [Release::Complete, Release::Failed, Release::Unknown] {
        let active = Arc::new(AtomicUsize::new(0));
        let mut root = session(&active);
        let mut child = session(&active);
        input(&mut child, outcome);
        append(&mut root, child);
        reap_children(&mut root);
        let unconfirmed = !matches!(outcome, Release::Complete);
        assert_eq!(root.input_cleanup_unconfirmed(), unconfirmed);
        // The parent itself reports no failure; child-cleanup flags must not contaminate the next parent-worker enable eligibility.
        assert_eq!(root.input_cleanup(), None);
        assert!(!root.input_cleanup_unknown);
        if unconfirmed {
            let mut next = session(&active);
            input(&mut next, Release::Complete);
            append(&mut root, next);
            reap_children(&mut root);
            assert!(
                root.input_cleanup_unconfirmed(),
                "later success cannot erase failure"
            );
        }
        let result = finish(&mut root);
        if unconfirmed {
            assert_eq!(
                result,
                Err("Live input cleanup failed or outcome unknown".into())
            );
        } else {
            assert_eq!(result, Ok(()));
        }
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn nested_retired_child_cannot_erase_unconfirmed_cleanup() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let mut root = session(&active);
    let mut child = session(&active);
    let mut nested = session(&active);
    input(&mut nested, Release::Failed);
    append(&mut child, nested);
    append(&mut root, child);
    reap_children(&mut root);
    assert!(root.input_cleanup_unconfirmed());
    assert_eq!(
        finish(&mut root),
        Err("Live input cleanup failed or outcome unknown".into())
    );
    assert_eq!(active.load(Ordering::Acquire), 0);
}
#[test]
fn capture_only_child_failure_does_not_poison_parent_cleanup() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let mut root = session(&active);
    let mut child = session(&active);
    child.error = Some("fixture child capture failure".into());
    input(&mut child, Release::Complete);
    append(&mut root, child);
    reap_children(&mut root);
    assert!(!root.input_cleanup_unconfirmed());
    assert_eq!(finish(&mut root), Ok(()));
    assert_eq!(active.load(Ordering::Acquire), 0);
}
