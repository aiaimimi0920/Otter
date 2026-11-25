//! Explicit stop for live documents: original Store revocation -> canonical pending -> original unlink/durable receipt -> canonical removal -> background ack.
use super::*;
use crate::session_source::Source;
use std::time::{Duration, Instant};
#[path = "application_projection_sender_stop_job.rs"]
mod job;
#[derive(Default)]
pub(super) struct Owner {
    pub busy: bool,
    job: Option<job::Job>,
    due: Option<Instant>,
    next: usize,
}
impl Owner {
    #[cfg(test)]
    pub(super) fn finished(&self) -> bool {
        self.job.as_ref().is_some_and(|job| job.finished())
    }
    pub(super) fn pending(&self) -> bool {
        self.job.is_some()
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.due
            .into_iter()
            .chain(self.job.as_ref().and_then(job::Job::deadline))
            .min()
    }
}
pub(super) struct Context {
    canonical: Source,
    original: Source,
    workspace: crate::workspace::IdentityStamp,
    scope: String,
}
impl Context {
    fn capture(app: &Application) -> Option<Self> {
        if !app.sender_stop_idle() {
            return None;
        }
        Some(Self {
            canonical: app.workspace_checkpoint.as_ref()?.sender_source()?.clone(),
            original: app.restored_session.as_ref()?.original.clone(),
            workspace: app.workspace.identity_stamp(),
            scope: app.projection_sender_scope()?.to_string(),
        })
    }
    fn matches(&self, app: &Application) -> bool {
        self.workspace.matches(&app.workspace)
            && Self::capture(app).is_some_and(|current| {
                self.canonical.same_storage(&current.canonical)
                    && self.original.same_storage(&current.original)
                    && self.scope == current.scope
            })
    }
}
impl Application {
    pub(super) fn sender_stop_idle(&self) -> bool {
        matches!(self.projection_sender.phase, Phase::Idle)
            && !self.remote_sessions.projection_pending()
            && self.sender_stop_available()
    }
    // Begin needs only an exact memory snapshot, without reading the Store lock; it can immediately interrupt Updating and persist revocation through the original background chain.
    pub(super) fn sender_stop_available(&self) -> bool {
        self.projection_sender.group.is_none()
            && (matches!(self.projection_sender.phase, Phase::Idle)
                && !self.remote_sessions.projection_pending()
                || matches!(self.projection_sender.phase, Phase::Updating(_)))
            && self.projection_checkpoint_available()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && !self.exit_requested.get()
            && !self.modals.blocked()
            && !self.display_composition.borrow().is_busy()
            && !self.projection_receive.busy()
            && restore_batch::guard::is_idle(self)
    }
    pub(in crate::desktop) unsafe fn start_projection_sender_stop(
        &mut self,
        selected: crate::desktop::device_authorization_ui::sender::StopSelection,
    ) {
        let request = self
            .workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_source())
            .filter(|source| source.same_storage(&selected.source))
            .filter(|_| {
                self.sender_stop_available()
                    && !self.projection_sender.stop.busy
                    && self.projection_sender.stop_group.is_none()
                    && !selected.binding.stopped()
                    && self.projection_sender_inclusion(&selected.binding)
                    && self.documents.iter().any(|doc| {
                        doc.is_alive() && doc.state.unit_id.as_str() == selected.binding.unit()
                    })
            })
            .and_then(|_| self.projection_sender.profile.entries())
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|request| selected.binding.matches_intent(request.intent()))
            })
            .cloned();
        if let Some(request) = request {
            self.stop_projection_sender();
            self.start_sender_stop_work(request, job::Work::Begin);
        } else {
            self.sender_status(crate::ui_text::text("app-projection-stop-intent-changed"));
        }
    }
    pub(super) fn begin_sender_stop_request(&mut self, request: Request) {
        self.start_sender_stop_work(request, job::Work::Begin);
    }
    fn start_sender_stop_work(&mut self, request: Request, work: job::Work) {
        match job::Job::start(request, work, self.execution_wake.callback()) {
            Ok(job) => {
                self.projection_sender.stop.job = Some(job);
                self.projection_sender.stop.busy = true;
            }
            Err(_) => self.sender_stop_retry(),
        }
    }
    fn sender_stop_retry(&mut self) {
        self.projection_sender.stop.due = Some(Instant::now() + Duration::from_secs(2));
        self.sender_status(crate::ui_text::text("app-projection-stop-incomplete"));
    }
    pub(super) unsafe fn poll_sender_stop(&mut self) {
        // Consume deadline wakeups; temporary workspace unavailability or unpersisted Begin must not leave expired WM_NULL spinning.
        if self
            .projection_sender
            .stop
            .due
            .is_some_and(|due| due <= Instant::now())
        {
            self.projection_sender.stop.due = None;
        }
        if self
            .projection_sender
            .stop
            .job
            .as_ref()
            .is_some_and(|job| job.finished())
        {
            let mut job = self.projection_sender.stop.job.take().unwrap();
            match job.join() {
                Ok(job::Output::Acknowledged) => {
                    self.projection_sender.profile.retired(&job.request);
                    self.sender_status(crate::ui_text::text("app-projection-sender-stopped"));
                }
                Ok(job::Output::Patched(context, source)) if context.matches(self) => {
                    if let Some(batch) = &mut self.projection_sender.stop_group {
                        batch.patched(&context.canonical, &context.original, &source);
                    }
                    self.restored_session
                        .as_mut()
                        .unwrap()
                        .edit_original(source);
                    self.saved_state = None;
                    self.sender_status(crate::ui_text::text("app-projection-stop-saving"));
                }
                Ok(job::Output::Begun | job::Output::Patched(..)) => {}
                Err(_) if job.ack && job.request.published_retired() => {
                    // persist may have actually committed but report an error because a recovery copy remains; do not leave a ghost entry in the original profile.
                    self.projection_sender.profile.retired(&job.request);
                    self.projection_sender.stop.due = None;
                    self.sender_status(crate::ui_text::text(
                        "app-projection-stop-recovery-retained",
                    ));
                }
                Err(_) if job.begin && !job.request.stop_requested() => {
                    // Without published revocation, do not pretend the request was accepted; preserve original Request and let the user explicitly retry through the menu.
                    self.projection_sender.stop.busy = false;
                    self.projection_sender.stop.due = None;
                    self.sender_status(crate::ui_text::text(
                        "app-projection-stop-authority-failed",
                    ));
                }
                Err(_) => self.sender_stop_retry(),
            }
        }
        if self.projection_sender.stop.job.is_some() {
            return;
        }
        self.process_projection_stop_group();
        if self.projection_sender.stop.job.is_some() {
            return;
        }
        let Some(context) = Context::capture(self) else {
            return;
        };
        let request = self
            .projection_sender
            .profile
            .entries()
            .and_then(|entries| {
                // One network failure must not leave other durably accepted group members waiting forever for pending/ack.
                entries
                    .iter()
                    .enumerate()
                    .cycle()
                    .skip(self.projection_sender.stop.next)
                    .take(entries.len())
                    .find(|(_, request)| {
                        request.stop_requested() && request.intent().scope() == context.scope
                    })
                    .map(|(index, request)| (index, request.clone()))
            });
        self.projection_sender.stop.busy = request.is_some();
        let Some((index, request)) = request else {
            return;
        };
        if self
            .projection_sender
            .stop
            .due
            .is_some_and(|due| due > Instant::now())
        {
            return;
        }
        self.projection_sender.stop.due = None;
        self.projection_sender.stop.next = index + 1;
        let owner = self.workspace_checkpoint.as_ref().unwrap();
        let Some(index) = owner
            .sender_index()
            .filter(|index| index.is_for(&context.canonical))
        else {
            return;
        };
        let binding = index.for_intent(request.intent()).cloned();
        let unlinked = request.published_unlinked();
        match binding {
            Some(binding) if !binding.stopped() || unlinked => {
                self.start_sender_stop_work(
                    request,
                    job::Work::Patch {
                        context,
                        binding,
                        remove: unlinked,
                    },
                );
            }
            Some(_) => {
                self.projection_sender.cleanup.enqueue_stop(request);
                self.sender_status(crate::ui_text::text("app-projection-stopping"));
            }
            None if unlinked => {
                if let Some(proof) =
                    crate::projection_delivery::sender::binding::stop::Acknowledgement::committed(
                        index,
                        &context.canonical,
                        &context.scope,
                        request.intent(),
                    )
                {
                    self.start_sender_stop_work(request, job::Work::Ack(proof));
                }
            }
            None => self.projection_sender.cleanup.enqueue_stop(request),
        }
    }
}
