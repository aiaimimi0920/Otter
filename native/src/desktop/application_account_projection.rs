//! v2 单贴纸接管复用真实 Bound/追加/检查点；网络接受、磁盘保存与可见呈现分别结算。
use super::*;
use crate::desktop::session_candidate::VectorCandidate;
use crate::projection_account::scene::Binding;
#[cfg(test)]
use crate::projection_account::scene::Index;
use std::sync::atomic::AtomicBool;
pub(super) struct Prepared {
    pub vector: VectorCandidate,
    pub retained: crate::pixels::Pixels,
    pub binding: Binding,
}
/// 发网前捕获的原工作区身份；不是授权，不按迟到回复重新选择锚点或当前场景。
pub(super) struct Destination {
    source: crate::session_source::Source,
    workspace: crate::workspace::IdentityStamp,
    guard: restore_batch::guard::Guard,
    consent: Option<crate::desktop::device_authorization_ui::policy::Stamp>,
    participation: crate::desktop::device_authorization_ui::policy::Participation,
    pub space: crate::session_placement::Space,
    pub scope: crate::session_projection_scope::Scope,
}
impl Destination {
    pub(super) fn source(&self) -> &crate::session_source::Source {
        &self.source
    }
    pub(super) unsafe fn capture(app: &Application) -> Result<Self> {
        Self::capture_for(app, false)
    }
    pub(super) unsafe fn capture_existing(app: &Application) -> Result<Self> {
        Self::capture_for(app, true)
    }
    unsafe fn capture_for(app: &Application, existing: bool) -> Result<Self> {
        if !app.projection_checkpoint_available()
            || app.restore_job.is_some()
            || app.save_job.is_some()
            || app.exit_requested.get()
            || app.modals.blocked()
            || app.workspace.units().len() >= crate::workspace::MAX_UNITS
        {
            return Err(stale());
        }
        let record = app.restored_session.as_ref().ok_or_else(stale)?;
        Ok(Self {
            source: record.original.clone(),
            workspace: app.workspace.identity_stamp(),
            guard: restore_batch::guard::Guard::capture(app)?,
            consent: if existing {
                None
            } else {
                Some(
                    app.authorization
                        .borrow()
                        .consent
                        .stamp()
                        .ok_or_else(stale)?,
                )
            },
            participation: app
                .authorization
                .borrow()
                .consent
                .participation()
                .ok_or_else(stale)?,
            space: record.space.ok_or_else(stale)?,
            scope: record.projection_scope.ok_or_else(stale)?,
        })
    }
    pub(super) unsafe fn matches(&self, app: &Application) -> bool {
        app.projection_checkpoint_available()
            && app.restore_job.is_none()
            && app.save_job.is_none()
            && !app.exit_requested.get()
            && !app.modals.blocked()
            && self.workspace.matches(&app.workspace)
            && self.guard.matches(app)
            && self
                .consent
                .as_ref()
                .is_none_or(|stamp| app.authorization.borrow().consent.matches(stamp))
            && app
                .authorization
                .borrow()
                .consent
                .participates(&self.participation)
            && app.restored_session.as_ref().is_some_and(|record| {
                record.original.same_storage(&self.source)
                    && record.projection_scope == Some(self.scope)
                    && restore_batch::append::same_space(record.space, self.space)
            })
    }
    /// 必须在后台准备；原会话追加器保留其它 Unit、链接、组及未知元数据。
    #[cfg(test)]
    pub(super) fn prepare(
        &self,
        vector: VectorCandidate,
        binding: Binding,
        units: &[crate::workspace_id::UnitId],
        cancel: &AtomicBool,
    ) -> Result<Prepared> {
        if binding.scope() != self.scope {
            return Err(stale());
        }
        let index =
            Index::capture(&vector.candidate.retained_source_record()).map_err(|_| stale())?;
        if !index.includes(&binding) {
            return Err(stale());
        }
        let retained = crate::session_append::prepare(
            &self.source,
            vector.candidate.original_json(),
            units,
            cancel,
        )
        .map_err(|message| Error::new(E_INVALIDARG, message))?;
        Ok(Prepared {
            vector,
            retained,
            binding,
        })
    }
}
pub(super) struct Receipt {
    pub binding: Binding,
    workspace: crate::workspace::IdentityStamp,
    source: crate::session_source::Source,
    consent: Option<crate::desktop::device_authorization_ui::policy::Stamp>,
    participation: crate::desktop::device_authorization_ui::policy::Participation,
}
impl Receipt {
    /// 保存不能恢复已关闭面板、暂停或换源前的呈现许可；磁盘回执仍然有效。
    pub(super) unsafe fn present(
        &self,
        app: &Application,
        cancel: Option<&AtomicBool>,
    ) -> Result<bool> {
        let view = app.authorization.borrow();
        let allowed = !view.closed
            && view.origin.as_deref() == Some(self.binding.origin())
            && view.consent.participates(&self.participation)
            && self
                .consent
                .as_ref()
                .is_none_or(|stamp| view.consent.matches(stamp));
        drop(view);
        if !allowed
            || cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
            || !self.saved(app)
        {
            return Ok(false);
        }
        let doc = app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == self.binding.unit())
            .ok_or_else(stale)?;
        projection::receive::present::Visual::capture(doc)?.present(doc)?;
        Ok(true)
    }
    /// 只查询真实提交时安装的后台索引，不在 UI 扫描会话、访问文件或验签。
    pub(super) fn saved(&self, app: &Application) -> bool {
        self.workspace.matches(&app.workspace)
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.source))
            && app
                .workspace_checkpoint
                .as_ref()
                .is_some_and(|owner| owner.includes_account(&self.binding, &app.workspace))
    }
}
impl Application {
    pub(super) unsafe fn publish_account_projection(
        &mut self,
        destination: &Destination,
        prepared: Prepared,
    ) -> Result<Receipt> {
        use crate::desktop::session_binding::Bound;
        if !destination.matches(self)
            || self
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == prepared.binding.unit())
        {
            return Err(stale());
        }
        let cancel = AtomicBool::new(false);
        let catalog = self.catalog.borrow().loaded.clone();
        let states = Bound::prepare_for(prepared.vector, catalog.clone(), &cancel)
            .map_err(|failure| failure.1)?
            .into_parts_for(catalog.as_ref(), &cancel)
            .map_err(|_| stale())?
            .prepare_states(&cancel)?;
        let windows = self.prepare_restored_batch(states, &cancel, |placement| {
            destination
                .space
                .physical(placement.rect)
                .map_err(|_| stale())
        })?;
        if !destination.matches(self) {
            return Err(stale());
        }
        let append = self.prepare_append(
            windows,
            destination.source.clone(),
            prepared.retained,
            destination.space,
        )?;
        // 先隐藏接管，真实检查点确认后才允许接收消费者显示；不把请求成功当作显示成功。
        self.publish_append(append, false)?;
        Ok(Receipt {
            binding: prepared.binding,
            workspace: self.workspace.identity_stamp(),
            source: self.restored_session.as_ref().unwrap().original.clone(),
            consent: destination.consent.clone(),
            participation: destination.participation.clone(),
        })
    }
}
fn stale() -> Error {
    Error::new(
        E_ABORT,
        "Account projection destination changed or unavailable",
    )
}
