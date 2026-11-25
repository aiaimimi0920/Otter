//! Group stop owns at most eight original Requests and source fences; original stop's single slot still performs durable revocation/patch/unlink/ack.
use super::*;
use crate::desktop::device_authorization_ui::sender::groups::StopSelection;
use crate::session_source::Source;
#[path = "application_projection_sender_stop_group_flow.rs"]
mod flow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Queued,
    Starting,
    Pending,
    Stopped,
    Failed,
    Cancelled,
}
struct Entry {
    binding: Binding,
    request: Option<Request>,
    state: State,
}
pub(super) struct Batch {
    selection: StopSelection,
    workspace: crate::workspace::IdentityStamp,
    scope: String,
    canonical: Source,
    original: Source,
    entries: Vec<Entry>,
    valid: bool,
    dirty: bool,
}
impl Batch {
    fn capture(app: &Application, selection: StopSelection) -> Option<Self> {
        let view = app.authorization.borrow();
        let current = app.workspace_checkpoint.as_ref()?.sender_source()?;
        if !app.sender_stop_available()
            || app.projection_sender.stop.busy
            || app.projection_sender.stop_group.is_some()
            || !current.same_storage(&selection.source)
            || !view.targets.directory.as_ref().is_some_and(|directory| {
                std::sync::Arc::ptr_eq(directory, &selection.group.directory)
            })
            || !crate::desktop::device_authorization_ui::sender::groups::stop_enabled(
                &view,
                &selection.group.unit,
                selection.group.index,
            )
        {
            return None;
        }
        let group = selection
            .group
            .directory
            .groups()?
            .get(selection.group.index)?;
        let scope = app.projection_sender_scope()?.to_string();
        let requests = app.projection_sender.profile.entries()?;
        let entries = app
            .workspace_checkpoint
            .as_ref()?
            .sender_index()?
            .entries()
            .iter()
            .filter(|binding| {
                binding.unit() == selection.group.unit.as_str()
                    && binding.scope() == scope
                    && binding.envelope().origin() == selection.group.directory.origin()
                    && group
                        .target_ids()
                        .iter()
                        .any(|id| Some(id.as_str()) == binding.target())
            })
            .map(|binding| Entry {
                binding: binding.clone(),
                request: requests
                    .iter()
                    .find(|request| binding.matches_intent(request.intent()))
                    .cloned(),
                state: State::Queued,
            })
            .collect::<Vec<_>>();
        if entries.is_empty()
            || entries.len() > crate::projection_delivery::sender::binding::MAX_PER_UNIT
        {
            return None;
        }
        Some(Self {
            workspace: app.workspace.identity_stamp(),
            scope,
            canonical: current.clone(),
            original: app.restored_session.as_ref()?.original.clone(),
            selection,
            entries,
            valid: true,
            dirty: true,
        })
    }
    unsafe fn matches(&mut self, app: &Application) -> bool {
        let view = app.authorization.borrow();
        let current = app
            .workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.sender_source());
        let valid = self.valid
            && self.workspace.matches(&app.workspace)
            && !view.closed
            && view.origin.as_deref() == Some(self.selection.group.directory.origin())
            && view.targets.directory.as_ref().is_some_and(|directory| {
                std::sync::Arc::ptr_eq(directory, &self.selection.group.directory)
            })
            && app.restore_job.is_none()
            && app
                .restored_session
                .as_ref()
                .and_then(|record| record.projection_scope)
                .is_some_and(|scope| scope.to_string() == self.scope)
            && app
                .documents
                .iter()
                .any(|doc| doc.is_alive() && doc.state.unit_id == self.selection.group.unit)
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.original))
            && current.is_some_and(|source| {
                source.same_storage(&self.canonical) || source.same_storage(&self.original)
            });
        if valid {
            self.canonical = current.unwrap().clone();
        }
        valid
    }
    // Only patches revalidated by the original stop Context may advance the fence; external byte-identical source content cannot be recaptured as a new baseline.
    pub(super) fn patched(&mut self, canonical: &Source, original: &Source, next: &Source) {
        self.valid &= original.same_storage(&self.original)
            && (canonical.same_storage(&self.canonical) || canonical.same_storage(&self.original));
        if self.valid {
            self.canonical = canonical.clone();
            self.original = next.clone();
        }
    }
    // Canonical encoding has its own allocation; register it only from an actual Checkpoint commit receipt, never by recapturing during polling.
    pub(super) fn committed(
        &mut self,
        index: Option<&Index>,
        canonical: &Source,
        original: &Source,
        workspace: &crate::workspace::Workspace,
    ) {
        self.valid &= self.workspace.matches(workspace)
            && original.same_storage(&self.original)
            && index.is_some_and(|index| index.is_for(canonical));
        if self.valid {
            self.canonical = canonical.clone();
        }
    }
    fn cancel(&mut self) {
        for entry in &mut self.entries {
            if entry.state == State::Queued {
                entry.state = State::Cancelled;
            }
        }
        self.dirty = true;
    }
    fn report(&mut self, app: &Application) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let count = |state| {
            self.entries
                .iter()
                .filter(|entry| entry.state == state)
                .count()
        };
        let group = &self.selection.group.directory.groups().unwrap()[self.selection.group.index];
        let mut view = app.authorization.borrow_mut();
        view.sender.group_stopping = true;
        view.sender.group_unit = Some(self.selection.group.unit.clone());
        view.sender.group_results = vec![crate::ui_text::format(
            "app-projection-stop-group-summary",
            &[
                ("group", group.name().into()),
                ("stopped", count(State::Stopped).into()),
                (
                    "pending",
                    (count(State::Starting) + count(State::Pending)).into(),
                ),
                ("failed", count(State::Failed).into()),
                (
                    "cancelled",
                    (count(State::Queued) + count(State::Cancelled)).into(),
                ),
            ],
        )];
        view.sender
            .group_results
            .extend(self.entries.iter().map(|entry| {
                let state = match entry.state {
                    State::Queued => crate::ui_text::text("app-projection-stop-queued"),
                    State::Starting => crate::ui_text::text("app-projection-stop-starting"),
                    State::Pending => crate::ui_text::text("app-projection-stop-pending"),
                    State::Stopped => crate::ui_text::text("app-projection-stop-confirmed"),
                    State::Failed => crate::ui_text::text("app-projection-stop-failed"),
                    State::Cancelled => crate::ui_text::text("app-projection-stop-cancelled"),
                };
                let name = if entry.binding.name().is_empty() {
                    entry
                        .binding
                        .target()
                        .unwrap_or(crate::ui_text::text("app-projection-share-invitation"))
                } else {
                    entry.binding.name()
                };
                crate::ui_text::format(
                    "app-projection-target-result",
                    &[("target", name.into()), ("status", state.into())],
                )
            }));
    }
}
