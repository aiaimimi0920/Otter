//! sender 清理先后台持久减权，再以当前原设备授权沿原 Job unlink；失败有界退避并保全部原文。
use super::*;
use crate::desktop::loom_execution_signal::{CompletionSignal, Notify};
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
    time::{Duration, Instant},
};
struct Entry {
    request: Request,
    marked: bool,
    failures: u8,
    due: Instant,
    stable_ready: bool,
}
struct Mark {
    request: Request,
    thread: Option<JoinHandle<std::result::Result<(), SenderError>>>,
}
#[derive(Default)]
pub(super) struct Owner {
    deleted: Vec<Binding>,
    entries: Vec<Entry>,
    mark: Option<Mark>,
    pub(super) network: Option<Request>,
    wake: Option<Instant>,
}
impl Owner {
    pub(super) fn enqueue(&mut self, request: Request, marked: bool) {
        if self
            .entries
            .iter()
            .any(|old| old.request.same_intent(&request))
        {
            return;
        }
        debug_assert!(self.entries.len() < crate::projection_delivery::sender::MAX_ENTRIES);
        self.entries.push(Entry {
            request,
            marked,
            failures: 0,
            due: Instant::now(),
            stable_ready: false,
        });
    }
    pub(super) fn enqueue_stop(&mut self, request: Request) {
        self.enqueue(request.clone(), true);
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.request.same_intent(&request))
        {
            entry.stable_ready = true;
        }
    }
    pub(super) fn pending_local(&self) -> bool {
        !self.deleted.is_empty()
            || self.mark.is_some()
            || self.entries.iter().any(|entry| !entry.marked)
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.wake
    }
    pub(super) fn finished(&mut self, request: &Request, success: bool) {
        if success {
            self.entries.retain(|old| !old.request.same_intent(request));
        } else if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.request.same_intent(request))
        {
            entry.due =
                Instant::now() + Duration::from_secs((1u64 << entry.failures.min(5)).min(30));
            entry.failures = entry.failures.saturating_add(1);
            self.wake = Some(self.wake.map_or(entry.due, |wake| wake.min(entry.due)));
        }
    }
    pub(super) fn resolve(&mut self, profile: &profile::Profile) {
        let Some(entries) = profile.entries() else {
            return;
        };
        for binding in std::mem::take(&mut self.deleted) {
            if let Some(request) = entries.iter().find(|request| {
                !request.stop_requested() && binding.matches_intent(request.intent())
            }) {
                self.enqueue(request.clone(), request.intent().cancelled());
            }
        }
    }
    fn poll_mark(&mut self, notify: Notify) {
        if self
            .mark
            .as_ref()
            .is_some_and(|mark| mark.thread.as_ref().unwrap().is_finished())
        {
            let mut mark = self.mark.take().unwrap();
            if mark
                .thread
                .take()
                .unwrap()
                .join()
                .unwrap_or(Err(SenderError::Write))
                .is_ok()
            {
                if let Some(entry) = self
                    .entries
                    .iter_mut()
                    .find(|entry| entry.request.same_intent(&mark.request))
                {
                    entry.marked = true;
                    entry.failures = 0;
                    entry.due = Instant::now();
                }
            } else {
                self.finished(&mark.request, false);
            }
        }
        if self.mark.is_none() {
            if let Some(entry) = self
                .entries
                .iter()
                .find(|entry| !entry.marked && entry.due <= Instant::now())
            {
                let request = entry.request.clone();
                let worker = request.clone();
                match std::thread::Builder::new()
                    .name("otter-sender-cancel".into())
                    .spawn(move || {
                        let _done = CompletionSignal::new(Arc::new(AtomicBool::new(false)), notify);
                        worker.cancel()
                    }) {
                    Ok(thread) => {
                        self.mark = Some(Mark {
                            request,
                            thread: Some(thread),
                        })
                    }
                    Err(_) => self.finished(&request, false),
                }
            }
        }
        self.wake = self
            .entries
            .iter()
            .filter(|entry| !entry.marked && self.mark.is_none())
            .map(|entry| entry.due)
            .min();
    }
    pub(super) fn flush(&mut self) -> std::result::Result<(), SenderError> {
        self.network = None;
        if let Some(mut mark) = self.mark.take() {
            mark.thread
                .take()
                .unwrap()
                .join()
                .unwrap_or(Err(SenderError::Write))?;
            if let Some(entry) = self
                .entries
                .iter_mut()
                .find(|entry| entry.request.same_intent(&mark.request))
            {
                entry.marked = true;
            }
        }
        if !self.deleted.is_empty() {
            return Err(SenderError::Read);
        }
        for entry in &mut self.entries {
            if !entry.marked {
                entry.request.cancel()?;
                entry.marked = true;
            }
        }
        self.wake = None;
        Ok(())
    }
}
impl Drop for Mark {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl Application {
    pub(super) fn process_sender_cleanup(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let owner = &mut self.projection_sender;
        // discard/closed 会过滤 Event，但原 Job 已回收；不能留下幽灵清理身份吞掉后续 Send。
        if !self.remote_sessions.projection_pending() {
            if let Some(request) = owner.cleanup.network.take() {
                let retired = request.published_retired();
                owner
                    .cleanup
                    .finished(&request, retired || request.published_unlinked());
                if retired {
                    owner.profile.retired(&request);
                }
            }
        }
        owner.cleanup.resolve(&owner.profile);
        owner.cleanup.poll_mark(self.execution_wake.callback());
        if !matches!(owner.phase, Phase::Idle)
            || owner.cleanup.pending_local()
            || owner.cleanup.network.is_some()
            || self.remote_sessions.projection_pending()
            || self.projection_receive.busy()
            || self.modals.blocked()
            || self.restore_job.is_some()
            || self.exit_requested.get()
        {
            return;
        }
        let view = self.authorization.borrow();
        if view.closed || view.busy || view.projection.busy || view.targets.busy {
            return;
        }
        let Some(origin) = view.origin.as_deref() else {
            return;
        };
        let Ok(lease) = self
            .authorization_owner
            .lease(origin, now().unwrap_or(u64::MAX))
        else {
            return;
        };
        let instant = Instant::now();
        let selected = owner
            .cleanup
            .entries
            .iter()
            .find(|entry| {
                entry.marked
                    && (!entry.request.stop_requested() || entry.stable_ready)
                    && !entry.request.published_unlinked()
                    && entry.due <= instant
                    && entry.request.intent().envelope().origin() == origin
                    && entry.request.intent().envelope().source_device() == lease.device_id()
            })
            .map(|entry| entry.request.clone());
        let Some(request) = selected else {
            owner.cleanup.wake = owner
                .cleanup
                .entries
                .iter()
                .filter(|entry| {
                    entry.marked
                        && entry.due > instant
                        && entry.request.intent().envelope().origin() == origin
                })
                .map(|entry| entry.due)
                .min();
            return;
        };
        let prepared = config(origin);
        drop(view);
        if prepared.ok().is_some_and(|config| {
            self.remote_sessions
                .cancel_sender(config, lease, request.clone())
                .is_ok()
        }) {
            owner.cleanup.network = Some(request);
        } else {
            owner.cleanup.finished(&request, false);
        }
    }
    pub(in crate::desktop) unsafe fn collect_deleted_senders(&mut self) {
        if matches!(self.projection_sender.profile, profile::Profile::Disabled)
            || self.exit_requested.get()
            || self.checkpoint_is_exiting()
        {
            return;
        }
        for doc in self.documents.iter().filter(|doc| !doc.is_alive()) {
            for binding in self
                .workspace_checkpoint
                .as_ref()
                .map(|owner| {
                    owner.sender_deleted_bindings(&self.workspace, doc.state.unit_id.as_str())
                })
                .unwrap_or_default()
            {
                if !self.projection_sender.cleanup.deleted.contains(&binding) {
                    self.projection_sender.cleanup.deleted.push(binding);
                }
            }
        }
    }
}
