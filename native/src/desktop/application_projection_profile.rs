//! Default-profile receive-journal lifecycle; open once in the background, and memory snapshots grant neither network nor display permission.
use crate::{
    desktop::{
        device_authorization_ui::projection::recovery::View,
        loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
    },
    projection_delivery::journal::{Store, Ticket},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

#[derive(Default)]
pub(in crate::desktop::application) enum Owner {
    #[default]
    Disabled,
    Opening(Job),
    Ready {
        store: Store,
        entries: Vec<Arc<Ticket>>,
        policy: Box<super::policy::Owner>,
    },
    Failed(String),
}
struct Loaded {
    store: Store,
    entries: Vec<Ticket>,
    policy: super::policy::Owner,
}
type Prepared = Result<Loaded, String>;
pub(in crate::desktop::application) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    thread: Option<JoinHandle<Prepared>>,
}
impl Owner {
    pub(in crate::desktop::application) fn entries(&self) -> Option<&[Arc<Ticket>]> {
        match self {
            Self::Ready { entries, .. } => Some(entries),
            _ => None,
        }
    }
    pub(in crate::desktop::application) fn start(&mut self, root: PathBuf, notify: Notify) {
        if !matches!(self, Self::Disabled) {
            return; // Workspace switches neither reopen the same profile's exclusive Store nor implicitly retry failures.
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-projection-profile".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                if signal.load(Ordering::Acquire) {
                    return Err(crate::ui_text::text("app-projection-recovery-stopped").into());
                }
                let store = Store::open(&root).map_err(|e| {
                    crate::ui_text::format(
                        "app-projection-recovery-unavailable",
                        &[("error", format!("{e:?}").into())],
                    )
                })?;
                let entries = store.pending().map_err(|e| {
                    crate::ui_text::format(
                        "app-projection-recovery-unavailable",
                        &[("error", format!("{e:?}").into())],
                    )
                })?;
                if signal.load(Ordering::Acquire) {
                    return Err(crate::ui_text::text("app-projection-recovery-stopped").into());
                }
                let policy = super::policy::Owner::load(&root, &signal);
                Ok(Loaded {
                    store,
                    entries,
                    policy,
                })
            });
        *self = match worker {
            Ok(thread) => Self::Opening(Job {
                cancel,
                ready,
                thread: Some(thread),
            }),
            Err(error) => Self::Failed(crate::ui_text::format(
                "app-projection-recovery-start-failed",
                &[("error", error.to_string().into())],
            )),
        };
    }
    pub(in crate::desktop::application) fn poll(&mut self) {
        self.collect(false);
    }
    /// Application terminal state only: deleting the final Unit still requires the original Store; Opening is not an empty journal.
    pub(in crate::desktop::application) fn finish_open(&mut self) {
        self.collect(true);
    }
    pub(in crate::desktop::application) fn reap_deadline(&self) -> Option<std::time::Instant> {
        match self {
            Self::Opening(job) => reap_deadline(&job.ready),
            Self::Ready { policy, .. } => policy.reap_deadline(),
            _ => None,
        }
    }
    fn collect(&mut self, terminal: bool) {
        let Self::Opening(job) = self else { return };
        if !terminal && !job.thread.as_ref().unwrap().is_finished() {
            return;
        }
        let result = job.thread.take().unwrap().join().unwrap_or_else(|_| {
            Err(crate::ui_text::text("app-projection-recovery-task-exited").into())
        });
        *self = match result {
            Ok(loaded) => Self::Ready {
                store: loaded.store,
                entries: loaded.entries.into_iter().map(Arc::new).collect(),
                policy: Box::new(loaded.policy),
            },
            Err(error) => Self::Failed(error),
        };
    }
    pub(in crate::desktop::application) fn contains(&self, ticket: &Arc<Ticket>) -> bool {
        matches!(self, Self::Ready { entries, .. } if entries.iter().any(|old| Arc::ptr_eq(old, ticket)))
    }
    pub(in crate::desktop::application) fn recorded(&self, ticket: &Ticket) -> bool {
        matches!(self, Self::Ready { entries, .. } if entries.iter().any(|old| old.same_record(ticket)))
    }
    /// Synchronize Store's in-memory commit results only after task reaping; canceled completion notifications must not revive retired entries.
    pub(in crate::desktop::application) fn refresh(&mut self) {
        let Self::Ready { store, entries, .. } = self else {
            return;
        };
        match store.pending() {
            Ok(current) => {
                entries.retain(|old| current.iter().any(|ticket| ticket.same_record(old)));
                for ticket in current {
                    if !entries.iter().any(|old| ticket.same_record(old)) {
                        entries.push(Arc::new(ticket));
                    }
                }
            }
            Err(error) => {
                *self = Self::Failed(crate::ui_text::format(
                    "app-projection-recovery-unavailable",
                    &[("error", format!("{error:?}").into())],
                ))
            }
        }
    }
    pub(in crate::desktop::application) fn sync(
        &self,
        view: &mut View,
        mut included: impl FnMut(&Ticket) -> bool,
    ) {
        let entries = match self {
            Self::Ready { entries, .. } => entries.as_slice(),
            _ => &[],
        };
        if view.entries.len() != entries.len()
            || !view
                .entries
                .iter()
                .zip(entries)
                .all(|(a, b)| Arc::ptr_eq(a, b))
        {
            view.entries.clear();
            view.entries.extend_from_slice(entries);
        }
        // saved serves only identity lookup; update entries in one pass and clone only changed items.
        let mut count = 0;
        for entry in entries.iter().filter(|entry| included(entry)) {
            match view.saved.get_mut(count) {
                Some(old) if !Arc::ptr_eq(old, entry) => old.clone_from(entry),
                None => view.saved.push(entry.clone()),
                _ => {}
            }
            count += 1;
        }
        view.saved.truncate(count);
        let status = match self {
            Self::Opening(_) => crate::ui_text::text("app-projection-unsettled-reading"),
            Self::Failed(error) => error,
            _ => "",
        };
        if view.status != status {
            view.status = status.into();
        }
    }
    pub(in crate::desktop::application) fn close(&mut self) {
        *self = Self::Disabled;
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // Filesystem calls cannot be forcibly aborted; wait for reaping only at application terminal state, leaving no ownerless threads.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[test]
fn projection_profile_reaps_only_after_completed_notifier_exits() {
    use crate::projection_delivery::journal::tests::Directory;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    let directory = Directory::new();
    let mut owner = Owner::Disabled;
    let (notified, notification) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let blocked = std::sync::Mutex::new(blocked);
    owner.start(
        directory.0,
        Arc::new(move || {
            notified.send(()).unwrap();
            let _ = blocked.lock().unwrap().recv_timeout(Duration::from_secs(5));
        }),
    );
    let release = release; // On assertion failure, disconnect notification before entering the owner's synchronous Drop.
    notification.recv_timeout(Duration::from_secs(5)).unwrap();
    owner.poll();
    assert!(matches!(owner, Owner::Opening(_)));
    assert!(owner.reap_deadline().is_some());
    release.send(()).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while matches!(owner, Owner::Opening(_)) {
        owner.poll();
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(matches!(owner, Owner::Ready { .. }));
    assert!(owner.reap_deadline().is_none());
}
