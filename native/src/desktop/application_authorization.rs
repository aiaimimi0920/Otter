//! Only the application loop accesses the authorization owner; shared menu state provides only bounded requests and credential-free results.
use super::*;
use crate::{
    desktop::device_authorization_ui::Request,
    device_authorization_config::{Action, Config},
    device_authorization_runtime::Task,
};
#[path = "application_authorization_events.rs"]
mod events;

impl Application {
    pub(super) fn process_authorization(&mut self) {
        if !crate::elevation_status::allows_desktop_integrations() {
            self.authorization.borrow_mut().request = None;
            return;
        }
        let stop_preference_failed = self.process_local_loom();
        self.process_authorization_with_projection_status(Config::from_env, stop_preference_failed);
    }
    #[cfg(test)]
    pub(super) fn process_authorization_with_projection(
        &mut self,
        config: impl FnMut(
            &str,
        )
            -> std::result::Result<Config, crate::device_authorization_config::Error>,
    ) {
        self.process_authorization_with_projection_status(config, false);
    }
    fn process_authorization_with_projection_status(
        &mut self,
        mut config: impl FnMut(
            &str,
        )
            -> std::result::Result<Config, crate::device_authorization_config::Error>,
        stop_preference_failed: bool,
    ) {
        unsafe {
            self.process_account_projection();
        }
        self.projection_profile.poll();
        self.live_viewer_requests(&mut config);
        unsafe {
            self.projection_edit_requests(&mut config);
        }
        self.process_projection_policy();
        self.projection_profile.sync(
            &mut self.authorization.borrow_mut().projection.recovery,
            |ticket| self.projection_checkpoint_inclusion(ticket).is_some(),
        );
        let resume = {
            let mut view = self.authorization.borrow_mut();
            if matches!(view.request, Some(Request::ResumeSurface(_))) {
                match view.request.take() {
                    Some(Request::ResumeSurface(id)) => Some(id),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(id) = resume {
            self.resume_surface_batch(&id);
        }
        // Dialogs must not hold View's RefCell borrow across message-loop turns.
        let confirmation = {
            let mut view = self.authorization.borrow_mut();
            if matches!(view.request, Some(Request::ConfirmSurface(_))) {
                match view.request.take() {
                    Some(Request::ConfirmSurface(id)) => Some(id),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(id) = confirmation {
            if !self.confirm_surface_batch(&id) {
                self.confirm_surface_event(&id);
            }
        }
        let display = {
            let mut view = self.authorization.borrow_mut();
            if matches!(
                view.request,
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Display(_)
                ))
            ) {
                match view.request.take() {
                    Some(Request::Projection(
                        crate::desktop::device_authorization_ui::projection::Request::Display(
                            ticket,
                        ),
                    )) => Some(ticket),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some(ticket) = display {
            unsafe {
                self.start_projection_display_recovery(ticket, &mut config);
            }
        }
        // Scope may allocate a persistent identity once for an old workspace; do not retain View's RefCell borrow across this step.
        let receive_kind = {
            let view = self.authorization.borrow();
            match &view.request {
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Reattach(entry),
                )) if !self
                    .workspace
                    .units()
                    .iter()
                    .any(|unit| unit.as_str() == entry.receiver_unit_id()) =>
                {
                    Some(true)
                }
                Some(Request::Projection(
                    crate::desktop::device_authorization_ui::projection::Request::Reattach(_),
                )) => None,
                Some(Request::Projection(_)) => Some(false),
                _ => None,
            }
        };
        let receive_scope = unsafe {
            match receive_kind {
                Some(true) => super::projection::receive::Scope::capture_existing(self).ok(),
                Some(false) => super::projection::receive::Scope::capture(self).ok(),
                None => None,
            }
        };
        self.projection_sender_requests(&mut config);
        let invalidate_sender = {
            let view = self.authorization.borrow();
            matches!(view.request, Some(Request::Cancel | Request::Start(_)))
                || matches!(&view.request, Some(Request::Select(origin)) if view.origin.as_ref() != Some(origin))
        };
        if invalidate_sender {
            self.stop_projection_sender();
        }
        // Release the View borrow before consuming OpenArt; the local chain uses an explicit HTTPS origin, not the old remote attach path.
        let local_request = {
            let mut view = self.authorization.borrow_mut();
            if !view.closed && matches!(view.request, Some(Request::OpenArt { .. })) {
                match view.request.take() {
                    Some(Request::OpenArt { art, node }) => Some((view.origin.clone(), art, node)),
                    _ => unreachable!(),
                }
            } else {
                None
            }
        };
        if let Some((origin, art, node)) = local_request {
            let result = origin
                .ok_or_else(|| crate::ui_text::text("app-service-not-selected").to_owned())
                .and_then(|origin| {
                    let configuration = config(&origin)
                        .map_err(|_| crate::ui_text::text("app-art-service-invalid").to_owned())?;
                    self.open_local_art(&origin, art, &node, configuration)
                });
            let mut view = self.authorization.borrow_mut();
            view.mount_busy = self.remote_sessions.attaching() || self.local_arts.busy();
            view.status = match result {
                Ok(()) => crate::ui_text::text("app-art-opening-locally").into(),
                Err(error) => error,
            };
        }
        let retire_local = {
            let view = self.authorization.borrow();
            matches!(&view.request, Some(Request::Cancel | Request::Start(_)))
                || matches!(&view.request,Some(Request::Select(origin)) if view.origin.as_ref()!=Some(origin))
        };
        // Existing offline leases are not part of Device sessions; explicit authorization/clearing/service changes must stop local HWNDs without a view borrow.
        if retire_local {
            self.local_arts.reset();
            self.remote_sessions.cancel_viewers();
        }
        let mut view = self.authorization.borrow_mut();
        // Consume revocation/switches before collecting completed results; an old request cannot overwrite a new selection even if already complete.
        if let Some(request) = view.request.take().filter(|_| !view.closed) {
            match request {
                Request::AccountProjection(_) => {
                    unreachable!("account requests consumed before borrowing view")
                }
                Request::WatchLive(_) | Request::StopLive(_) => {
                    unreachable!("viewer requests consumed before borrowing view")
                }
                Request::SendProjection(_)
                | Request::ShareProjection(_)
                | Request::SendProjectionGroup(_)
                | Request::StopProjectionGroup(_)
                | Request::CancelProjectionSend
                | Request::StopProjectionSend(_)
                | Request::RetryProjectionSend(_) => {
                    unreachable!("sender requests consumed before borrowing view")
                }
                Request::SubmitProjectionEdit(_)
                | Request::ResolveProjectionEdit(_, _)
                | Request::SetProjectionEditMode(_, _)
                | Request::InitializeProjectionEdit(_)
                | Request::CancelProjectionEdit => {
                    unreachable!("edit requests consumed before borrowing view")
                }
                Request::ProjectionTargets => {
                    if self.modals.blocked()
                        || self.exit_requested.get()
                        || self.projection_receive.busy()
                        || self.projection_edit.busy()
                    {
                        view.targets.status =
                            crate::ui_text::text("app-target-catalog-app-busy").into();
                    } else {
                        super::projection::targets::start(
                            &mut view,
                            &mut self.authorization_owner,
                            &mut self.remote_sessions,
                            &mut config,
                        );
                    }
                }
                Request::Projection(request) => {
                    if self.modals.blocked()
                        || self.exit_requested.get()
                        || self.restore_job.is_some()
                        || self.projection_receive.busy()
                        || self.projection_edit.busy()
                    {
                        view.projection.status = crate::ui_text::text("app-projection-workspace-switching").into();
                    } else if matches!(
                        request,
                        crate::desktop::device_authorization_ui::projection::Request::Accept(_)
                            | crate::desktop::device_authorization_ui::projection::Request::Import(_)
                            | crate::desktop::device_authorization_ui::projection::Request::Reattach(
                                _
                            )
                    ) && receive_scope.is_none()
                    {
                        view.projection.status = crate::ui_text::text("app-projection-workspace-unsafe").into();
                    } else if super::projection::start(
                        &mut view,
                        request,
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                        &self.projection_profile,
                        &mut config,
                    ) {
                        self.projection_receive.waiting(receive_scope);
                    }
                }
                Request::ConfirmSurface(_) | Request::ResumeSurface(_) => {
                    unreachable!("confirmation consumed before borrowing view")
                }
                Request::ReceivePolicy(_) | Request::ReceivePause(_) => {
                    unreachable!("policy consumed before borrowing view")
                }
                Request::RestoreSurface(candidate) => {
                    let result = super::surface_remount::start(
                        &mut view,
                        candidate,
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                        &self.surface_windows,
                        self.live_sessions.len(),
                        &mut config,
                    );
                    view.mount_busy = self.remote_sessions.attaching();
                    view.restoring = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => crate::ui_text::text("app-recovery-remounting").into(),
                        Err(error) => error,
                    };
                }
                Request::LoadRecovery(path) => {
                    view.recovery.clear();
                    view.recovery_candidate = None;
                    let result = self.remote_sessions.read_recovery_file(path);
                    view.recovery_busy = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => crate::ui_text::text("app-recovery-reading").into(),
                        Err(error) => crate::ui_text::format(
                            "app-recovery-file-not-read",
                            &[("error", error.to_string().into())],
                        ),
                    };
                }
                Request::QueryRecovery(record) => {
                    view.recovery_candidate = None;
                    let result = super::surface_recovery::query(
                        &record,
                        view.origin.as_deref(),
                        &mut self.authorization_owner,
                        &mut self.remote_sessions,
                        &mut config,
                    );
                    view.recovery_busy = result.is_ok();
                    view.recovery_status = match result {
                        Ok(()) => crate::ui_text::text("app-recovery-querying").into(),
                        Err(error) => error,
                    };
                }
                Request::OpenArt { .. } => {
                    unreachable!("local request consumed before borrowing view")
                }
                Request::Select(origin) => {
                    view.connected = false;
                    view.loom_id = None;
                    if view.origin.as_ref() != Some(&origin) {
                        super::projection::reset(
                            &mut self.remote_sessions,
                            &mut view,
                            &mut self.projection_receive,
                            &mut self.projection_cleanup,
                        );
                        view.recovery_candidate = None;
                        view.recovery_status.clear();
                        self.remote_sessions.cancel_discovery();
                        self.remote_sessions.cancel_surface_requests();
                        view.catalog = None;
                        view.catalog_busy = false;
                        if let Some(old) = &view.origin {
                            let _ = self.authorization_owner.invalidate(old);
                        }
                        view.origin = Some(origin);
                        view.busy = false;
                        view.status =
                            crate::ui_text::text("app-service-selected-no-session").into();
                    }
                }
                Request::Cancel => {
                    view.connected = false;
                    view.loom_id = None;
                    super::projection::reset(
                        &mut self.remote_sessions,
                        &mut view,
                        &mut self.projection_receive,
                        &mut self.projection_cleanup,
                    );
                    view.recovery_candidate = None;
                    view.recovery_status.clear();
                    self.remote_sessions.cancel_discovery();
                    self.remote_sessions.cancel_surface_requests();
                    view.catalog = None;
                    view.catalog_busy = false;
                    if let Some(origin) = &view.origin {
                        let _ = self.authorization_owner.invalidate(origin);
                    }
                    view.busy = false;
                    if !stop_preference_failed {
                        view.status = crate::ui_text::text("app-session-cleared").into();
                    }
                    // Old sessions without local Art trust do not touch the profile; configuration capture performs no I/O, and revocation commits through the existing bounded background owner.
                    if let Some(origin) = view
                        .origin
                        .as_deref()
                        .filter(|o| self.art_issuers.is_trusted(o))
                    {
                        let result = config(origin)
                            .map_err(|_| {
                                crate::ui_text::text("app-offline-clear-invalid-config").to_owned()
                            })
                            .and_then(|config| {
                                self.authorization_owner
                                    .start(Task::ClearOffline { config })
                                    .map_err(|e| {
                                        crate::ui_text::format(
                                            "app-offline-clear-not-started",
                                            &[("error", format!("{e:?}").into())],
                                        )
                                    })
                            });
                        match result {
                            Ok(()) => {
                                view.busy = true;
                                view.status =
                                    crate::ui_text::text("app-offline-clear-started").into();
                            }
                            Err(error) => view.status = error,
                        }
                    }
                }
                Request::Discover => {
                    view.catalog = None;
                    let result = view
                        .origin
                        .as_deref()
                        .ok_or_else(|| crate::ui_text::text("app-service-not-selected").to_owned())
                        .and_then(|origin| {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_err(|_| {
                                    crate::ui_text::text("app-system-time-invalid").to_owned()
                                })?;
                            let now = u64::try_from(now.as_millis()).map_err(|_| {
                                crate::ui_text::text("app-system-time-out-of-range").to_owned()
                            })?;
                            let lease =
                                self.authorization_owner.lease(origin, now).map_err(|e| {
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
                            self.remote_sessions.discover(config, lease).map_err(|e| {
                                crate::ui_text::format(
                                    "app-catalog-task-not-started",
                                    &[("error", e.to_string().into())],
                                )
                            })
                        });
                    view.catalog_busy = result.is_ok();
                    view.status = match result {
                        Ok(()) => crate::ui_text::text("app-remote-catalog-querying").into(),
                        Err(error) => error,
                    };
                }
                Request::Start(action) => {
                    super::projection::reset(
                        &mut self.remote_sessions,
                        &mut view,
                        &mut self.projection_receive,
                        &mut self.projection_cleanup,
                    );
                    view.recovery_candidate = None;
                    view.recovery_status.clear();
                    self.remote_sessions.cancel_discovery();
                    self.remote_sessions.cancel_surface_requests();
                    view.catalog = None;
                    view.catalog_busy = false;
                    if let Some(origin) = &view.origin {
                        let result = Config::from_env(origin)
                            .map_err(|e| {
                                crate::ui_text::format(
                                    "app-configuration-invalid-detail",
                                    &[("error", format!("{e:?}").into())],
                                )
                            })
                            .and_then(|config| {
                                self.authorization_owner
                                    .start(Task::Configured { config, action })
                                    .map_err(|e| {
                                        crate::ui_text::format(
                                            "app-task-not-started",
                                            &[("error", format!("{e:?}").into())],
                                        )
                                    })
                            });
                        match result {
                            Ok(()) => {
                                view.busy = true;
                                view.status = match action {
                                    Action::RequestPairing => {
                                        crate::ui_text::text("app-pairing-submitting")
                                    }
                                    Action::AwaitApproval => {
                                        crate::ui_text::text("app-pairing-awaiting-approval")
                                    }
                                    Action::Renew => {
                                        crate::ui_text::text("app-authorization-requesting")
                                    }
                                }
                                .into();
                            }
                            Err(error) => {
                                view.busy = false;
                                view.status = error;
                            }
                        }
                    }
                }
            }
        }
        events::refresh(&mut self.authorization_owner, &mut view);
        drop(view);
        self.poll_projection_inbox();
        unsafe {
            self.poll_projection_sender(&mut config);
        }
        self.poll_projection_receive(&mut config);
        unsafe {
            self.poll_projection_edit(&mut config);
        }
        let now = std::time::Instant::now();
        self.poll_projection_cleanup(now);
        // An overdue inbox gets the shared slot before another background unlink retry.
        unsafe { self.process_projection_automatic(&mut config, now) };
        self.dispatch_projection_cleanup(&mut config, now);
        unsafe { self.process_projection_sync(&mut config, std::time::Instant::now()) };
        self.refresh_projection_targets(&mut config);
        self.projection_profile.sync(
            &mut self.authorization.borrow_mut().projection.recovery,
            |ticket| self.projection_checkpoint_inclusion(ticket).is_some(),
        );
        unsafe {
            crate::desktop::device_authorization_ui::share::refresh(&self.authorization);
        }
    }
}

#[cfg(test)]
#[path = "application_authorization_tests.rs"]
mod tests;
