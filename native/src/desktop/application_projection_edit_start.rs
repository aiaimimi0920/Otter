//! 菜单请求消费与发网准入；释放View借用后才捕获来源/租约，恢复原pending由后台完成。
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
                self.edit_status("协作提交已取消；原请求保留供显式重试");
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
        }
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
            let view = self.authorization.borrow();
            if self.projection_edit.busy()
                || self.modals.blocked()
                || view.closed
                || view.busy
                || self.exit_requested.get()
                || self.restore_job.is_some()
                || self.save_job.is_some()
                || self.projection_receive.busy()
                || !self.sender_idle_for_edit()
                || self.remote_sessions.projection_pending()
                || !self.projection_checkpoint_available()
            {
                return Err(stale());
            }
            drop(view);
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
            Ok(Pending {
                fence,
                job,
                cancelled: false,
                remaining,
            })
        })();
        match result {
            Ok(pending) => {
                self.projection_edit
                    .1
                    .block(&self.workspace, &selection.unit);
                self.authorization.borrow_mut().editing.unit = Some(selection.unit);
                self.projection_edit.0 = Phase::Sending(Box::new(pending));
                self.edit_status("正在提交协作修改…");
                true
            }
            Err(error) => {
                self.edit_status(&format!("协作提交未启动：{error}"));
                false
            }
        }
    }
    pub(super) fn edit_status(&self, status: &str) {
        let mut view = self.authorization.borrow_mut();
        view.editing.busy = self.projection_edit.busy();
        view.editing.status = status.into();
    }
}
