//! UI只捕获来源/目标/租约；PNG/初始wire/持久stage与网络都在既有Job worker。
use super::*;
impl Application {
    pub(in crate::desktop::application) unsafe fn start_projection_source(
        &mut self,
        unit: UnitId,
        selected: Option<Vec<String>>,
        explicit: bool,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) -> bool {
        let result = (|| {
            let view = self.authorization.borrow();
            if self.projection_edit.busy()
                || !self.sender_idle_for_edit()
                || self.modals.blocked()
                || view.closed
                || view.busy
                || view.projection.busy
                || view.targets.busy
                || self.projection_receive.busy()
            {
                return Err(stale());
            }
            let origin = view.origin.clone().ok_or_else(stale)?;
            drop(view);
            let mut targets = self.projection_source_targets(&unit, &origin);
            if let Some(selected) = &selected {
                targets.retain(|target| {
                    selected.contains(&target.binding.envelope().projection_id().to_owned())
                });
            }
            if targets.is_empty() {
                return Err(stale());
            }
            let (scope, snapshot) =
                super::super::super::projection_sender::scope::Scope::capture(self, &unit)?;
            let doc = self
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == unit)
                .ok_or_else(stale)?;
            let canvas = doc.state.canvas.borrow();
            let existing = doc.state.projection_edit.borrow();
            if existing.as_ref().is_some_and(|owner| {
                let snapshot = owner.capture(&canvas.editor);
                snapshot.suspended().is_some()
                    || snapshot.role() != crate::projection_edit::workspace::Role::Source
                    || snapshot.context().origin() != origin
            }) {
                return Err(stale());
            }
            let base = Base::capture(&canvas.editor);
            let stamp = existing.as_ref().map(|owner| owner.stamp());
            let expected_session = existing.as_ref().map(|owner| {
                owner
                    .capture(&canvas.editor)
                    .document()
                    .session_id()
                    .to_owned()
            });
            drop(existing);
            drop(canvas);
            let scheduled = (!explicit && targets.len() == 1)
                .then(|| targets[0].binding.envelope().projection_id().to_owned());
            let cached = expected_session.is_some()
                && scheduled
                    .as_deref()
                    .is_some_and(|id| self.projection_source_cached(&unit, id));
            let prepared = config(&origin).map_err(|_| stale())?;
            let current = self
                .authorization_owner
                .lease(&origin, now()?)
                .map_err(|_| stale())?;
            let lease = self
                .authorization_owner
                .lease(&origin, now()?)
                .map_err(|_| stale())?;
            if !current.same_grant(&lease) {
                return Err(stale());
            }
            current.notify_revocation(self.execution_wake.callback());
            let context = Context::new(scope.scope, unit.as_str(), &origin, current.device_id())
                .map_err(|_| stale())?;
            let ids = targets
                .iter()
                .map(|target| target.binding.envelope().projection_id().to_owned())
                .collect();
            let source = scope.binding_source.clone();
            let render_source = source.clone();
            let render_unit = unit.clone();
            let input = flow::Input {
                root: self
                    .workspace_checkpoint
                    .as_ref()
                    .ok_or_else(stale)?
                    .profile_root()
                    .to_owned(),
                context: context.clone(),
                targets,
                source,
                explicit,
                cached,
                expected_session,
                prepare: Box::new(move |lease, anchor, session| {
                    snapshot.freeze(&render_source, render_unit.as_str(), anchor, session, lease)
                }),
            };
            let job = Job::start_edit_source(prepared, lease, input).map_err(|_| stale())?;
            Ok(Pending {
                guard: Guard {
                    unit: unit.clone(),
                    workspace: self.workspace.identity_stamp(),
                    context,
                    current,
                    stamp,
                    targets: ids,
                    explicit,
                    scheduled,
                    target: None,
                },
                job,
                source: Some((scope, base)),
                cancelled: false,
            })
        })();
        match result {
            Ok(pending) => {
                self.authorization.borrow_mut().editing.unit = Some(unit);
                self.projection_edit.0 =
                    super::super::Phase::Source(Box::new(Phase::Sending(pending)));
                self.edit_status("正在初始化 / 同步源端协作…");
                true
            }
            Err(error) => {
                self.edit_status(&format!("源端协作未启动：{error}"));
                false
            }
        }
    }
}
