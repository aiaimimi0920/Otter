//! Menu-request consumption and network admission; capture source/lease only after releasing View, and recover original pending state in the background.
use super::*;
impl Application {
    pub(in crate::desktop::application) unsafe fn projection_edit_requests(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        let request = {
            let mut view = self.authorization.borrow_mut();
            if matches!(
                view.request,
                Some(
                    Request::SubmitProjectionEdit(_)
                        | Request::InitializeProjectionEdit(_)
                        | Request::ResolveProjectionEdit(_, _)
                        | Request::SetProjectionEditMode(_, _)
                        | Request::CancelProjectionEdit
                )
            ) {
                view.request.take()
            } else {
                None
            }
        };
        match request {
            Some(Request::InitializeProjectionEdit(unit)) => {
                self.start_projection_source(unit, None, true, config);
            }
            Some(Request::SubmitProjectionEdit(selection)) => {
                self.start_projection_edit(selection, config, 8, Action::Submit);
            }
            Some(Request::ResolveProjectionEdit(selection, keep_local)) => {
                self.start_projection_edit(selection, config, 8, Action::Resolve { keep_local });
            }
            Some(Request::SetProjectionEditMode(selection, mode)) => {
                self.start_projection_edit(selection, config, 8, Action::Mode(mode));
            }
            Some(Request::CancelProjectionEdit) => {
                self.projection_edit.stop();
                self.edit_error("协作提交已取消；原请求保留供显式重试");
            }
            _ => {}
        }
        let invalidate = {
            let view = self.authorization.borrow();
            matches!(view.request, Some(Request::Cancel | Request::Start(_)))
                || matches!(&view.request, Some(Request::Select(origin)) if view.origin.as_ref() != Some(origin))
        };
        if invalidate {
            self.projection_edit.stop();
        } else if self.projection_edit_available() {
            let view = self.authorization.borrow();
            let idle = !view.closed
                && !view.busy
                && !view.targets.busy
                && !view.projection.busy
                && view.request.is_none();
            drop(view);
            if idle {
                let choice = self
                    .documents
                    .iter()
                    .filter(|doc| doc.is_alive())
                    .find_map(|doc| {
                        let mut preference = doc.state.projection_mode.borrow_mut();
                        let choice = preference.as_mut()?;
                        let owner = doc.state.projection_edit.borrow();
                        let owner = owner.as_ref()?;
                        let canvas = doc.state.canvas.borrow();
                        let snapshot = owner.authoritative();
                        if !choice.stamp.same_document(&canvas.editor)
                            || choice.mode == snapshot.document().mode()
                        {
                            preference.take();
                            return None;
                        }
                        if choice.submitted
                            || snapshot.role() != crate::projection_edit::workspace::Role::Source
                            || !owner.can_submit(&canvas.editor)
                        {
                            return None;
                        }
                        // Dispatch explicit direction only once; retain while background work is busy, with failures retried explicitly by the user.
                        choice.submitted = true;
                        Some((
                            Selection {
                                unit: doc.state.unit_id.clone(),
                                stamp: owner.stamp(),
                            },
                            choice.mode,
                        ))
                    });
                if let Some((selection, mode)) = choice {
                    self.authorization.borrow_mut().editing.unit = Some(selection.unit.clone());
                    self.start_projection_edit(selection, config, 1, Action::Mode(mode));
                }
            }
        }
    }
    fn projection_edit_available(&self) -> bool {
        let view = self.authorization.borrow();
        !self.projection_edit.busy()
            && !self.account_projection.busy()
            && !self.modals.blocked()
            && !view.closed
            && !view.busy
            && !self.exit_requested.get()
            && self.restore_job.is_none()
            && self.save_job.is_none()
            && !self.projection_receive.busy()
            && self.sender_idle_for_edit()
            && !self.remote_sessions.projection_pending()
            && self.projection_checkpoint_available()
    }
    pub(super) unsafe fn start_projection_edit(
        &mut self,
        selection: Selection,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
        remaining: u8,
        action: Action,
    ) -> bool {
        let result = (|| {
            let available = self.projection_edit_available();
            let automatic = matches!(action, Action::Automatic);
            if !available && !automatic {
                return Err(stale());
            }
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.is_alive() && doc.state.unit_id == selection.unit)
                .ok_or_else(stale)?;
            let owner = doc.state.projection_edit.borrow();
            let owner = owner
                .as_ref()
                .filter(|owner| owner.stamp() == selection.stamp)
                .ok_or_else(stale)?;
            let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
            if automatic && !crate::projection_delivery_runtime::edit::automatic(&snapshot) {
                return Ok(None);
            }
            if !available {
                return Err(stale());
            }
            if snapshot.suspended().is_some()
                || self
                    .restored_session
                    .as_ref()
                    .and_then(|record| record.projection_scope)
                    != Some(snapshot.context().scope())
            {
                return Err(stale());
            }
            let destination = if snapshot.role()
                == crate::projection_edit::workspace::Role::Receiver
            {
                let receiver = self
                    .projection_profile
                    .entries()
                    .unwrap_or(&[])
                    .iter()
                    .find(|ticket| {
                        ticket.unit() == selection.unit.as_str()
                            && ticket.origin() == snapshot.context().origin()
                            && ticket.device() == snapshot.context().device()
                            && ticket.envelope().projection_id() == snapshot.projection_id()
                    })
                    .cloned()
                    .ok_or_else(stale)?;
                let binding = self
                    .projection_sync_index()
                    .and_then(|index| index.for_ticket(&receiver))
                    .ok_or_else(stale)?;
                if binding.stopped() || binding.digest() != snapshot.document().basis().digest() {
                    return Err(stale());
                }
                Destination::Receiver(receiver)
            } else {
                let target = self
                    .projection_source_targets(&selection.unit, snapshot.context().origin())
                    .into_iter()
                    .find(|target| {
                        target.binding.envelope().projection_id() == snapshot.projection_id()
                    })
                    .ok_or_else(stale)?;
                if target.binding.digest() != snapshot.document().basis().digest()
                    || target.binding.edit_route() != snapshot.route()
                {
                    return Err(stale());
                }
                Destination::Source(Box::new(target.binding))
            };
            let origin = snapshot.context().origin();
            let configuration = config(origin).map_err(|_| stale())?;
            let current = self
                .authorization_owner
                .lease(origin, now()?)
                .map_err(|_| stale())?;
            let lease = self
                .authorization_owner
                .lease(origin, now()?)
                .map_err(|_| stale())?;
            if current.device_id() != snapshot.context().device() || !current.same_grant(&lease) {
                return Err(stale());
            }
            current.notify_revocation(self.execution_wake.callback());
            let fence = Fence {
                unit: selection.unit.clone(),
                workspace: self.workspace.identity_stamp(),
                stamp: selection.stamp,
                destination,
                current,
            };
            let input = Input {
                root: self
                    .workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .profile_root()
                    .to_owned(),
                snapshot,
                op_id: format!(
                    "op:{}",
                    &crate::session_projection_scope::Scope::generate()?.to_string()[..32]
                ),
                action,
            };
            let job = Job::start_edit(configuration, lease, input).map_err(|_| stale())?;
            Ok(Some(Pending {
                fence,
                job,
                cancelled: false,
                remaining,
            }))
        })();
        match result {
            Ok(Some(pending)) => {
                self.projection_edit
                    .1
                    .block(&self.workspace, &selection.unit);
                self.authorization.borrow_mut().editing.unit = Some(selection.unit);
                self.projection_edit.0 = Phase::Sending(Box::new(pending));
                self.edit_status("正在提交协作修改…");
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.edit_error(&format!("协作提交未启动：{error}"));
                false
            }
        }
    }
    pub(super) fn edit_error(&self, status: &str) {
        let unit = self.authorization.borrow().editing.unit.clone();
        if let Some(doc) = self
            .documents
            .iter()
            .find(|doc| Some(&doc.state.unit_id) == unit.as_ref())
        {
            let mut preference = doc.state.projection_mode.borrow_mut();
            if preference.as_ref().is_some_and(|choice| choice.submitted) {
                // Return to the actual mode after UI intent fails; the original recovery flow still owns persisted requests.
                preference.take();
            }
        }
        self.edit_status(status);
        self.authorization.borrow_mut().editing.recovery = true;
    }
    pub(super) fn edit_status(&self, status: &str) {
        let mut view = self.authorization.borrow_mut();
        view.editing.busy = self.projection_edit.busy();
        view.editing.status = status.into();
        view.editing.recovery = false;
    }
}
