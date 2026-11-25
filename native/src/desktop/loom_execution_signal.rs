//! Background-task completion fence; notify after the final local owner drops so early UI polling cannot miss completion.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) type Notify = Arc<dyn Fn() + Send + Sync>;

/// Notification precedes thread exit. Owners without Win32 wait slots briefly recheck after notification,
/// joining only exited threads; do not enable polling while work remains unfinished or idle.
pub(crate) fn reap_deadline(ready: &AtomicBool) -> Option<std::time::Instant> {
    ready
        .load(Ordering::Acquire)
        .then(|| std::time::Instant::now() + std::time::Duration::from_millis(10))
}

pub(crate) struct CompletionSignal {
    ready: Arc<AtomicBool>,
    notify: Notify,
}
impl CompletionSignal {
    /// Declare before task inputs, permits, and network resources so reverse destruction publishes last.
    pub(crate) fn new(ready: Arc<AtomicBool>, notify: Notify) -> Self {
        Self { ready, notify }
    }
}
impl Drop for CompletionSignal {
    fn drop(&mut self) {
        self.ready.store(true, Ordering::Release);
        // Notification failures/third-party callback panics must not cause a second panic in unwinding workers.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_publishes_after_resources_and_before_notification() {
        struct Resource(Arc<AtomicBool>);
        impl Drop for Resource {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        for panic in [false, true] {
            let ready = Arc::new(AtomicBool::new(false));
            let released = Arc::new(AtomicBool::new(false));
            let (tx, rx) = std::sync::mpsc::channel();
            let observed = ready.clone();
            let resource = released.clone();
            let notify = Arc::new(move || {
                tx.send((
                    observed.load(Ordering::Acquire),
                    resource.load(Ordering::Acquire),
                ))
                .unwrap();
            });
            let worker = std::thread::spawn(move || {
                let _signal = CompletionSignal::new(ready, notify);
                let _resource = Resource(released);
                assert!(!panic, "injected worker panic");
            });
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap(),
                (true, true)
            );
            assert_eq!(worker.join().is_err(), panic);
        }
    }
    #[test]
    fn notification_panic_does_not_escape_drop() {
        let ready = Arc::new(AtomicBool::new(false));
        drop(CompletionSignal::new(
            ready.clone(),
            Arc::new(|| panic!("injected notifier panic")),
        ));
        assert!(ready.load(Ordering::Acquire));
    }
}
