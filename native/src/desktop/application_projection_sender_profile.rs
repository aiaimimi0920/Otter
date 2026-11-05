//! sender outbox 只在后台打开一次；恢复快照不授重发权，持久取消项只进入减权队列。
use super::*;
use crate::desktop::loom_execution_signal::{CompletionSignal, Notify};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
#[derive(Default)]
pub(super) enum Profile {
    #[default]
    Disabled,
    Opening(Opening),
    Ready {
        root: PathBuf,
        store: Store,
        entries: Vec<Request>,
    },
    Failed,
}
type Opened = std::result::Result<(PathBuf, Store, Vec<Request>), SenderError>;
pub(super) struct Opening {
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<Opened>>,
}
impl Profile {
    pub(super) fn start(&mut self, root: PathBuf, notify: Notify) {
        if !matches!(self, Self::Disabled) {
            return;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = std::thread::Builder::new()
            .name("otter-sender-profile".into())
            .spawn(move || {
                let _done = CompletionSignal::new(Arc::new(AtomicBool::new(false)), notify);
                let store = Store::open(&root, &signal)?;
                let entries = store
                    .pending(&signal)?
                    .into_iter()
                    .map(|intent| Request::new(intent, root.clone()))
                    .collect();
                Ok((root, store, entries))
            });
        *self = match worker {
            Ok(thread) => Self::Opening(Opening {
                cancel,
                thread: Some(thread),
            }),
            Err(_) => Self::Failed,
        };
    }
    pub(super) fn collect(&mut self, terminal: bool) -> bool {
        let Self::Opening(opening) = self else {
            return false;
        };
        if !terminal && !opening.thread.as_ref().unwrap().is_finished() {
            return false;
        }
        let result = opening
            .thread
            .take()
            .unwrap()
            .join()
            .unwrap_or(Err(SenderError::Read));
        *self = match result {
            Ok((root, store, entries)) => Self::Ready {
                root,
                store,
                entries,
            },
            Err(_) => Self::Failed,
        };
        true
    }
    pub(super) fn input(&self, root: &std::path::Path) -> Option<Store> {
        match self {
            Self::Ready {
                root: expected,
                store,
                ..
            } if expected == root => Some(store.clone()),
            _ => None,
        }
    }
    pub(super) fn entries(&self) -> Option<&[Request]> {
        match self {
            Self::Ready { entries, .. } => Some(entries),
            _ => None,
        }
    }
    pub(super) fn remember(&mut self, request: Request) {
        if let Self::Ready { entries, .. } = self {
            if !entries.iter().any(|old| old.same_intent(&request)) {
                debug_assert!(entries.len() < crate::projection_delivery::sender::MAX_ENTRIES);
                entries.push(request);
            }
        }
    }
    pub(super) fn retired(&mut self, request: &Request) {
        if let Self::Ready { entries, .. } = self {
            entries.retain(|old| !old.same_intent(request));
        }
    }
}
impl Drop for Opening {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
