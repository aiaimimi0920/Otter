//! Inbox-menu consumer at application settlement points; handle switching/revocation before results, without holding UI borrows across threads or disk operations.
use super::*;
use crate::desktop::{
    composition::relay::sessions::Sessions,
    device_authorization_ui::{projection as ui, View},
};
use crate::{
    device_authorization_config::Config,
    device_authorization_owner::Owner,
    projection_delivery_runtime::{Error as JobError, Output},
};
use std::sync::Arc;
#[path = "application_projection_cleanup.rs"]
pub(super) mod cleanup;
#[path = "application_projection_policy.rs"]
pub(super) mod policy;
#[path = "application_projection_profile.rs"]
pub(super) mod profile;
#[path = "application_projection_receive.rs"]
pub(super) mod receive;
#[path = "application_projection_targets.rs"]
pub(super) mod targets;

pub(super) fn reset(
    remote: &mut Sessions,
    view: &mut View,
    receive: &mut receive::Owner,
    cleanup: &mut cleanup::Owner,
) {
    receive.stop(cleanup);
    remote.cancel_projection();
    ui::reset(view);
    crate::desktop::device_authorization_ui::targets::reset(view);
}
pub(super) fn start(
    view: &mut View,
    request: ui::Request,
    owner: &mut Owner,
    remote: &mut Sessions,
    profile: &profile::Owner,
    config: &mut impl FnMut(
        &str,
    )
        -> std::result::Result<Config, crate::device_authorization_config::Error>,
) -> bool {
    if view.closed
        || view.account.busy
        || view.busy
        || view.projection.busy
        || view.targets.busy
        || remote.projection_pending()
    {
        view.projection.status =
            crate::ui_text::text("app-projection-inbox-task-unavailable").into();
        return false; // Neither overwrite busy state for a running task nor clear its candidate.
    }
    let refresh = matches!(request, ui::Request::Refresh);
    if refresh {
        view.projection.entries.clear();
    }
    let result = (|| {
        if !matches!(
            request,
            ui::Request::Compensate(_) | ui::Request::Display(_)
        ) {
            view.projection.participation = Some(
                view.consent
                    .participation()
                    .ok_or(crate::ui_text::text("app-projection-reception-paused"))?,
            );
        }
        if matches!(request, ui::Request::Accept(_) | ui::Request::Import(_))
            && view.consent.stamp().is_none()
        {
            return Err(crate::ui_text::text("app-projection-policy-denied").into());
        }
        let origin = view
            .origin
            .as_deref()
            .ok_or_else(|| crate::ui_text::text("app-service-not-selected").to_owned())?;
        if let ui::Request::Reject(entry) | ui::Request::Accept(entry) = &request {
            if entry.accepted()
                || entry.origin() != origin
                || !view
                    .projection
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err(crate::ui_text::text("app-projection-choice-expired").into());
            }
        }
        if let ui::Request::Reattach(entry) = &request {
            if !entry.accepted()
                || entry.origin() != origin
                || !view
                    .projection
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err(crate::ui_text::text("app-projection-recovery-choice-expired").into());
            }
        }
        if let ui::Request::Compensate(entry) = &request {
            if entry.origin() != origin
                || !profile.contains(entry)
                || !view
                    .projection
                    .recovery
                    .entries
                    .iter()
                    .any(|current| Arc::ptr_eq(current, entry))
            {
                return Err(crate::ui_text::text("app-projection-unsettled-choice-expired").into());
            }
        }
        let now = crate::desktop::surface_windows::now_ms()
            .map_err(|_| crate::ui_text::text("app-system-time-invalid").to_owned())?;
        let lease = owner.lease(origin, now).map_err(|e| {
            crate::ui_text::format(
                "app-authorization-required-detail",
                &[("error", format!("{e:?}").into())],
            )
        })?;
        let config = config(origin).map_err(|e| {
            crate::ui_text::format(
                "app-configuration-invalid-detail",
                &[("error", format!("{e:?}").into())],
            )
        })?;
        match request {
            ui::Request::Import(request) => {
                if !request.current() {
                    return Err(crate::ui_text::text("app-projection-invitation-cancelled").into());
                }
                let profile::Owner::Ready { store, .. } = profile else {
                    return Err(crate::ui_text::text("app-projection-storage-not-ready").into());
                };
                remote.import_projection(config, lease, request, store.clone())
            }
            ui::Request::Display(_) => {
                return Err(
                    crate::ui_text::text("app-projection-display-recovery-verification").into(),
                )
            }
            ui::Request::Refresh => remote.query_projection_inbox(config, lease),
            ui::Request::Reject(entry) => remote.reject_projection(config, lease, entry),
            ui::Request::Compensate(entry) => remote.compensate_projection(config, lease, entry),
            ui::Request::Accept(entry) => {
                let profile::Owner::Ready { store, .. } = profile else {
                    return Err(crate::ui_text::text("app-projection-storage-not-ready").into());
                };
                remote.accept_projection(config, lease, entry, store.clone())
            }
            ui::Request::Reattach(entry) => {
                let profile::Owner::Ready { store, .. } = profile else {
                    return Err(crate::ui_text::text("app-projection-storage-not-ready").into());
                };
                remote.reattach_projection(config, lease, entry, store.clone())
            }
        }
        .map_err(|e| {
            crate::ui_text::format(
                "app-projection-task-start-failed",
                &[("error", e.to_string().into())],
            )
        })
    })();
    let started = result.is_ok();
    view.projection.busy = started;
    view.projection.status = match result {
        Ok(()) => if refresh {
            crate::ui_text::text("app-projection-inbox-querying")
        } else {
            crate::ui_text::text("app-projection-processing")
        }
        .into(),
        Err(error) => error,
    };
    started
}
impl Application {
    /// Query only actual-publication/canonical-startup-restoration indexes; Unit existence and identical source content are merely current-scope fences.
    pub(super) fn projection_checkpoint_inclusion(
        &self,
        ticket: &crate::projection_delivery::journal::Ticket,
    ) -> Option<&crate::projection_delivery::binding::Binding> {
        if self.restore_job.is_some() {
            return None;
        }
        let owner = self.workspace_checkpoint.as_ref()?;
        let (index, source) = owner.projection_baseline()?;
        if !self
            .workspace
            .units()
            .iter()
            .any(|unit| unit.as_str() == ticket.unit())
            || self
                .restored_session
                .as_ref()
                .is_none_or(|record| !record.original.same_storage(source))
        {
            return None;
        }
        index.for_ticket(ticket)
    }
    pub(super) fn reset_projection_inbox(&mut self) {
        self.stop_projection_sender();
        reset(
            &mut self.remote_sessions,
            &mut self.authorization.borrow_mut(),
            &mut self.projection_receive,
            &mut self.projection_cleanup,
        );
    }
    pub(super) fn poll_projection_inbox(&mut self) {
        let pending = self.remote_sessions.projection_pending();
        let targets_pending = self.remote_sessions.projection_targets_pending();
        let event = self.remote_sessions.poll_projection();
        if pending && !targets_pending && !self.remote_sessions.projection_pending() {
            self.projection_profile.refresh();
        }
        self.collect_projection_abandoned(); // Must precede closed/origin/discard UI filtering.
        self.collect_projection_sender();
        let Some(event) = event else {
            return;
        };
        if event.sender {
            unsafe {
                self.finish_projection_sender(event.result, event.sender_cleanup);
            }
            return;
        }
        if event.targets {
            targets::finish(
                &mut self.authorization.borrow_mut(),
                &event.origin,
                event.result,
            );
            return;
        }
        if let Ok(Output::Received(ready)) = event.result {
            self.receive_projection_ready(*ready);
            return;
        }
        self.projection_receive.finish_metadata();
        let mut view = self.authorization.borrow_mut();
        if view.closed || view.origin.as_deref() != Some(event.origin.as_str()) {
            return;
        }
        view.projection.busy = false;
        if matches!(event.result, Ok(Output::Inbox(_) | Output::Rejected))
            && !view
                .projection
                .participation
                .take()
                .is_some_and(|stamp| view.consent.participates(&stamp))
        {
            return; // Pause/resume or failed writes must not revive old inbox snapshots; leave compensation and saved continuous sync untouched.
        }
        // Cleanup has its own retry schedule; its result is not an inbox attempt.
        if !event.compensation && event.result.is_ok() {
            if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                policy
                    .schedule
                    .completed(&event.origin, std::time::Instant::now());
            }
        }
        match event.result {
            Ok(Output::Targets(_)) => unreachable!("targets are consumed by the directory owner"),
            Ok(Output::Inbox(inbox)) => {
                let offline = inbox.has_offline_peers();
                view.projection.entries = inbox.into_entries().into_iter().map(Arc::new).collect();
                if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                    policy.schedule.inbox(&view.projection.entries);
                }
                view.projection.status = crate::ui_text::format(
                    if offline {
                        "app-projection-inbox-count-offline"
                    } else {
                        "app-projection-inbox-count"
                    },
                    &[("count", view.projection.entries.len().into())],
                );
            }
            Ok(Output::Rejected) => {
                if let Some(entry) = event.invitation {
                    view.projection
                        .entries
                        .retain(|current| !Arc::ptr_eq(current, &entry));
                }
                view.projection.status = crate::ui_text::text("app-projection-rejected").into();
            }
            Ok(Output::Received(_)) => {
                unreachable!("received owner consumed before borrowing view")
            }
            Ok(Output::Compensated) => {
                view.projection.status =
                    crate::ui_text::text("app-projection-unlink-confirmed").into();
            }
            Ok(Output::Displayed) => unreachable!("displayed is consumed by the receive owner"),
            Ok(Output::Synchronized(_)) => unreachable!("sync is consumed by the receive owner"),
            Ok(Output::Edited(_) | Output::EditAcknowledged) => {
                unreachable!("edit results require the collaboration owner")
            }
            Ok(Output::Sent(_) | Output::SenderUpdated(_) | Output::SenderUnlinked) => {
                unreachable!("sender results require their own consumer")
            }
            Err(JobError::Stopped) => {
                view.projection.entries.clear();
                view.projection.status =
                    crate::ui_text::text("app-projection-operation-cancelled").into();
            }
            Err(error) => {
                let mut retrying = false;
                if let profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
                    if !event.compensation {
                        policy
                            .schedule
                            .failure(&event.origin, std::time::Instant::now());
                    }
                    if error == JobError::AcceptBusyUnapplied {
                        retrying = event
                            .invitation
                            .as_deref()
                            .is_some_and(|entry| policy.schedule.accept_unapplied(entry));
                    }
                }
                view.projection.status = match error {
                    JobError::AcceptBusyUnapplied if retrying => {
                        crate::ui_text::text("app-projection-loom-busy-retry").into()
                    }
                    JobError::AcceptBusyUnapplied => {
                        crate::ui_text::text("app-projection-loom-busy-unapplied").into()
                    }
                    _ => crate::ui_text::format(
                        "app-projection-operation-failed",
                        &[("error", format!("{error:?}").into())],
                    ),
                };
            }
        }
    }
    pub(super) fn collect_projection_abandoned(&mut self) {
        if let Some(ticket) = self.remote_sessions.take_projection_abandoned() {
            self.projection_cleanup.abandon(ticket);
        }
    }
}
