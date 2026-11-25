//! One slot per frozen-gallery worker kind, shared by all menus in an application.
use crate::desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify};
use std::{
    cell::RefCell,
    io,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};

pub(super) struct Slot<T>(Rc<RefCell<State<T>>>);
enum State<T> {
    Idle,
    Active,
    Retired(Thread<T>),
}
struct Thread<T> {
    handle: Option<JoinHandle<T>>,
    ready: Arc<AtomicBool>,
}
pub(super) struct Job<T> {
    slot: Slot<T>,
    thread: Option<Thread<T>>,
    pub cancel: Arc<AtomicBool>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(State::Idle)))
    }
}
impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> Slot<T> {
    pub fn reap(&self) {
        let mut state = self.0.borrow_mut();
        if matches!(&*state, State::Retired(thread) if thread.is_finished()) {
            // Only a terminated worker is joined during the application pump.
            *state = State::Idle;
        }
    }
    pub fn deadline(&self) -> Option<Instant> {
        match &*self.0.borrow() {
            State::Retired(thread) => reap_deadline(&thread.ready),
            _ => None,
        }
    }
}
impl<T: Send + 'static> Slot<T> {
    pub(super) fn start(
        &self,
        name: &str,
        notify: Option<Notify>,
        work: impl FnOnce(&AtomicBool) -> T + Send + 'static,
    ) -> io::Result<Option<Job<T>>> {
        self.reap();
        let mut state = self.0.borrow_mut();
        if !matches!(*state, State::Idle) {
            return Ok(None);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let signal = ready.clone();
        let handle = std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let _completion = notify.map(|notify| CompletionSignal::new(signal, notify));
                work(&stop)
            })?;
        *state = State::Active;
        Ok(Some(Job {
            slot: self.clone(),
            thread: Some(Thread {
                handle: Some(handle),
                ready,
            }),
            cancel,
        }))
    }
}
impl<T> Job<T> {
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_some_and(Thread::is_finished)
    }
    pub fn poll(&mut self) -> Option<std::thread::Result<T>> {
        if !self.is_finished() {
            return None;
        }
        let result = self.thread.take().unwrap().handle.take().unwrap().join();
        *self.slot.0.borrow_mut() = State::Idle;
        Some(result)
    }
}
impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancel.store(true, Ordering::Release);
            *self.slot.0.borrow_mut() = State::Retired(thread);
        }
    }
}
impl<T> Thread<T> {
    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }
}
impl<T> Drop for Thread<T> {
    fn drop(&mut self) {
        // Normal reaping checks is_finished. If the whole owner dies, drain the
        // cancelled worker here rather than detaching it.
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn menu_close_cancels_without_joining_and_reopen_keeps_the_same_capacity() {
        let owner = Slot::default();
        let menu = owner.clone();
        let limit = Duration::from_secs(5);
        let (release_work, wait_work) = mpsc::channel();
        let (release_notify, wait_notify) = mpsc::channel();
        let wait_notify = std::sync::Mutex::new(wait_notify);
        let (notify, notification) = mpsc::channel();
        let callback = Arc::new(move || {
            notify.send(()).unwrap();
            wait_notify.lock().unwrap().recv_timeout(limit).unwrap();
        });
        let job = menu
            .start("test-gallery-close", Some(callback), move |cancel| {
                wait_work.recv_timeout(limit).unwrap();
                assert!(cancel.load(Ordering::Acquire));
                17
            })
            .unwrap()
            .unwrap();
        let cancel = job.cancel.clone();
        drop(job);
        drop(menu);
        assert!(cancel.load(Ordering::Acquire));
        assert!(owner.deadline().is_none());
        release_work.send(()).unwrap();
        notification.recv_timeout(limit).unwrap();
        assert!(owner.deadline().is_some());
        owner.reap();
        assert!(owner
            .start("forbidden-replacement", None, |_| panic!("over capacity"))
            .unwrap()
            .is_none());
        release_notify.send(()).unwrap();
        let end = Instant::now() + limit;
        while owner.deadline().is_some() {
            assert!(Instant::now() < end);
            owner.reap();
            std::thread::yield_now();
        }
        let mut replacement = owner
            .start("test-gallery-reopen", None, |_| 42)
            .unwrap()
            .unwrap();
        let end = Instant::now() + limit;
        loop {
            if let Some(result) = replacement.poll() {
                assert_eq!(result.unwrap(), 42);
                break;
            }
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
    }
}
