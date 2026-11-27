//! Real send admission outside callbacks; PNG/signing/full-session validation and binding candidates all run in the original Job's single background thread.
use super::*;
use crate::{
    desktop::device_authorization_ui::sender::Selection,
    device_authorization_config::Config,
    projection_delivery_runtime::{Error as JobError, SenderInput},
};
#[path = "application_projection_sender_choice.rs"]
mod choice;
use choice::Choice;
impl Application {
    pub(in crate::desktop) unsafe fn start_projection_share(
        &mut self,
        selection: crate::desktop::device_authorization_ui::share::SourceSelection,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.start_sender_choice(Choice::Invitation(selection), Vec::new(), config);
    }
    pub(in crate::desktop) unsafe fn start_projection_sender(
        &mut self,
        selection: Selection,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        self.start_projection_sender_admitted(selection, Vec::new(), config);
    }
    pub(super) unsafe fn start_projection_sender_admitted(
        &mut self,
        selection: Selection,
        admission: Vec<crate::projection_delivery::sender::Selection>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> bool {
        self.start_sender_choice(Choice::Device(selection), admission, config)
    }
    unsafe fn start_sender_choice(
        &mut self,
        selection: Choice,
        admission: Vec<crate::projection_delivery::sender::Selection>,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> bool {
        let result = (|| {
            if self.account_projection.busy()
                || self.projection_sender.busy()
                || self.projection_edit.busy()
                || self.projection_sender.pending_local()
                || self.projection_sender.cleanup.network.is_some()
                || self.remote_sessions.projection_pending()
                || self.projection_receive.busy()
            {
                return Err(scope::stale());
            }
            if !selection.valid(self) {
                return Err(scope::stale());
            }
            let (mut scope, snapshot) = scope::Scope::capture(self, selection.unit())?;
            if let Choice::Invitation(selection) = &selection {
                scope.cancel = Some(selection.cancel.clone());
            }
            let origin = selection.origin();
            let editing = self.projection_source_snapshot(selection.unit())?;
            if editing
                .as_ref()
                .is_some_and(|state| state.context().origin() != origin)
            {
                return Err(scope::stale());
            }
            let edit_root = self
                .workspace_checkpoint
                .as_ref()
                .ok_or_else(scope::stale)?
                .profile_root()
                .to_owned();
            {
                let view = self.authorization.borrow();
                if view.closed
                    || view.busy
                    || view.projection.busy
                    || view.targets.busy
                    || view.origin.as_deref() != Some(origin)
                {
                    return Err(scope::stale());
                }
            }
            let prepared_config = config(origin).map_err(|_| scope::stale())?;
            let root = prepared_config.identity_root().to_path_buf();
            let store = self
                .projection_sender
                .profile
                .input(&root)
                .ok_or_else(scope::stale)?;
            let current = self
                .authorization_owner
                .lease(origin, now()?)
                .map_err(|_| scope::stale())?;
            let lease = self
                .authorization_owner
                .lease(origin, now()?)
                .map_err(|_| scope::stale())?;
            lease.notify_revocation(self.execution_wake.callback());
            let name = selection.name()?.to_owned();
            let pending_name = name.clone();
            let selected = selection.prepared(&scope.scope.to_string())?;
            let source = scope.binding_source.clone();
            let unit = scope.unit.clone();
            self.remote_sessions
                .send_projection(
                    prepared_config,
                    lease,
                    Box::new(move |lease, track| {
                        Binding::admit_send(&source, &selected).map_err(JobError::Sender)?;
                        if !admission.is_empty() {
                            Binding::admit_batch(&source, &admission).map_err(JobError::Sender)?;
                        }
                        let frame = if let Some(editing) = &editing {
                            snapshot.frozen_send_frame(
                                &source,
                                unit.as_str(),
                                &edit_root,
                                editing,
                                lease,
                            )?
                        } else {
                            snapshot
                                .one_way(&source, unit.as_str(), lease.preparation_cancel())
                                .map_err(|_| JobError::Sender(SenderError::Snapshot))?
                        };
                        let intent = store
                            .prepare_tracked(
                                crate::projection_delivery::sender::Preparation {
                                    selection: &selected,
                                    frame,
                                    identity_root: &root,
                                    device: lease.device_id(),
                                    now_ms: now().map_err(|_| JobError::Stopped)?,
                                },
                                lease.preparation_cancel(),
                                |intent| track(Request::new(intent, root.clone())),
                            )
                            .map_err(JobError::Sender)?;
                        Ok(SenderInput {
                            request: Request::new(intent, root),
                            source,
                            name,
                        })
                    }),
                )
                .map_err(|_| scope::stale())?;
            self.projection_sender.phase = Phase::Waiting(Box::new(Pending {
                origin: origin.to_owned(),
                scope,
                current,
                request: None,
                stopped: false,
                name: pending_name,
            }));
            Ok(())
        })();
        self.sender_status(if result.is_ok() {
            crate::ui_text::text("app-projection-sending-invitation")
        } else {
            crate::ui_text::text("app-projection-send-unavailable")
        });
        result.is_ok()
    }
}
