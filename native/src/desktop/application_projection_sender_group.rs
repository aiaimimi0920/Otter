//! Device groups are only bounded queues of original single-target transactions; advance after each canonical confirmation without separately retaining pixels or network tasks.
use super::*;
use crate::desktop::device_authorization_ui::sender::groups::Selection;
use std::sync::Arc;
#[path = "application_projection_sender_group_flow.rs"]
mod flow;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Queued,
    Existing,
    Unavailable,
    Sending,
    Saved,
    Failed,
    Cancelled,
}
struct Entry {
    index: usize,
    state: State,
}
pub(super) struct Batch {
    selection: Selection,
    scope: scope::Scope,
    entries: Vec<Entry>,
    current: Option<usize>,
    unavailable: usize,
    dirty: bool,
    reported_retry: Option<bool>,
}
impl Batch {
    unsafe fn capture(app: &Application, selection: Selection) -> Result<Self> {
        let view = app.authorization.borrow();
        if !view
            .targets
            .directory
            .as_ref()
            .is_some_and(|directory| Arc::ptr_eq(directory, &selection.directory))
            || !crate::desktop::device_authorization_ui::sender::groups::enabled(
                &view,
                &selection.unit,
                selection.index,
            )
            || app.projection_sender.busy()
            || app.remote_sessions.projection_pending()
            || app.projection_sender.pending_local()
        {
            return Err(scope::stale());
        }
        let group = selection
            .directory
            .groups()
            .and_then(|groups| groups.get(selection.index))
            .ok_or_else(scope::stale)?;
        let unavailable = group.unavailable_count();
        let entries = selection
            .directory
            .targets()
            .iter()
            .enumerate()
            .filter(|(_, target)| group.target_ids().iter().any(|id| id == target.id()))
            .map(|(index, target)| Entry {
                index,
                state: if !target.route().available() {
                    State::Unavailable
                } else if crate::desktop::device_authorization_ui::sender::bound(
                    &view,
                    &selection.unit,
                    target.id(),
                ) {
                    State::Existing
                } else {
                    State::Queued
                },
            })
            .collect();
        drop(view);
        let (scope, _) = scope::Scope::capture(app, &selection.unit)?;
        Ok(Self {
            selection,
            scope,
            entries,
            current: None,
            unavailable,
            dirty: true,
            reported_retry: None,
        })
    }
    unsafe fn matches(&self, app: &Application) -> bool {
        let view = app.authorization.borrow();
        !view.closed
            && !view.busy
            && !view.projection.busy
            && !view.targets.busy
            && view.origin.as_deref() == Some(self.selection.directory.origin())
            && view
                .targets
                .directory
                .as_ref()
                .is_some_and(|directory| Arc::ptr_eq(directory, &self.selection.directory))
            && self.scope.matches(app)
            && now().is_ok_and(|now| {
                app.authorization_owner
                    .token(self.selection.directory.origin(), now)
                    .is_some()
            })
    }
    pub(super) fn committed(&mut self, saved: &Saved) {
        let Some(index) = self.current else { return };
        if !saved.updated
            && saved.expected.unit() == self.selection.unit.as_str()
            && saved.expected.target()
                == Some(self.selection.directory.targets()[self.entries[index].index].id())
            && saved.previous.same_storage(&self.scope.source)
        {
            self.entries[index].state = State::Saved;
            // Only this batch's original transaction may replace source content; do not recapture truth from arbitrary current content before save confirmation.
            self.scope.source = saved.source.clone();
            self.current = None;
            self.dirty = true;
        }
    }
    fn cancel(&mut self) {
        self.dirty = true;
        for entry in &mut self.entries {
            if matches!(entry.state, State::Queued | State::Sending) {
                entry.state = State::Cancelled;
            }
        }
        self.current = None;
    }
    fn report(&mut self, app: &Application) {
        let retry = matches!(app.projection_sender.phase, Phase::Retry(_));
        if !self.dirty && self.reported_retry == Some(retry) {
            return;
        }
        self.dirty = false;
        self.reported_retry = Some(retry);
        let group = &self.selection.directory.groups().unwrap()[self.selection.index];
        let saved = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.state, State::Saved | State::Existing))
            .count();
        let failed = self
            .entries
            .iter()
            .filter(|entry| entry.state == State::Failed)
            .count();
        let unavailable = self.unavailable
            + self
                .entries
                .iter()
                .filter(|entry| entry.state == State::Unavailable)
                .count();
        let cancelled = self
            .entries
            .iter()
            .filter(|entry| entry.state == State::Cancelled)
            .count();
        let mut view = app.authorization.borrow_mut();
        view.sender.group_stopping = false;
        view.sender.group_unit = Some(self.selection.unit.clone());
        view.sender.group_results = vec![crate::ui_text::format(
            "app-projection-send-group-summary",
            &[
                ("group", group.name().into()),
                ("saved", saved.into()),
                ("failed", failed.into()),
                ("unavailable", unavailable.into()),
                ("cancelled", cancelled.into()),
            ],
        )];
        view.sender
            .group_results
            .extend(self.entries.iter().map(|entry| {
                let target = &self.selection.directory.targets()[entry.index];
                let status = match entry.state {
                    State::Queued => crate::ui_text::text("app-projection-send-queued"),
                    State::Existing => crate::ui_text::text("app-projection-send-existing"),
                    State::Unavailable => {
                        crate::ui_text::text("app-projection-send-target-unavailable")
                    }
                    State::Sending if matches!(app.projection_sender.phase, Phase::Retry(_)) => {
                        crate::ui_text::text("app-projection-send-failed-retry")
                    }
                    State::Sending => crate::ui_text::text("app-projection-send-in-progress"),
                    State::Saved => crate::ui_text::text("app-projection-send-saved"),
                    State::Failed => crate::ui_text::text("app-projection-send-failed-stop"),
                    State::Cancelled => crate::ui_text::text("app-projection-send-cancelled"),
                };
                let name = if target.name().is_empty() {
                    target.id()
                } else {
                    target.name()
                };
                crate::ui_text::format(
                    "app-projection-target-result",
                    &[("target", name.into()), ("status", status.into())],
                )
            }));
    }
    fn admission(&self) -> Result<Vec<crate::projection_delivery::sender::Selection>> {
        self.entries
            .iter()
            .filter(|entry| entry.state == State::Queued)
            .map(|entry| {
                crate::projection_delivery::sender::Selection::for_target(
                    &self.scope.scope.to_string(),
                    self.selection.directory.origin(),
                    self.selection.unit.as_str(),
                    &self.selection.directory.targets()[entry.index],
                )
                .map_err(|_| scope::stale())
            })
            .collect()
    }
}
