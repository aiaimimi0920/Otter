//! 发网前的工作区来源围栏；不是接受授权，不能用响应到达时的新工作区补造旧目标。
use super::*;
use crate::{session_placement::Space, session_source::Source, session_window::Rect};
#[path = "application_projection_placement.rs"]
pub(super) mod placement;

pub(in crate::desktop::application) struct Scope {
    pub source: Source,
    pub space: Space,
    pub frame: Rect,
    pub projection_scope: crate::session_projection_scope::Scope,
    guard: restore_batch::guard::Guard,
    consent: Option<crate::desktop::device_authorization_ui::policy::Stamp>,
    participation: crate::desktop::device_authorization_ui::policy::Participation,
}
impl Scope {
    pub(in crate::desktop::application) unsafe fn capture(app: &mut Application) -> Result<Self> {
        Self::capture_for(app, false)
    }
    pub(in crate::desktop::application) unsafe fn capture_existing(
        app: &mut Application,
    ) -> Result<Self> {
        Self::capture_for(app, true)
    }
    unsafe fn capture_for(app: &mut Application, existing: bool) -> Result<Self> {
        if !app.projection_checkpoint_available()
            || app.save_job.is_some()
            || app.restore_job.is_some()
            || app.workspace.units().len() >= crate::workspace::MAX_UNITS
        {
            return Err(stale());
        }
        let record = app.restored_session.as_mut().ok_or_else(stale)?;
        if record.projection_scope.is_none() {
            record.projection_scope = Some(crate::session_projection_scope::Scope::generate()?);
        }
        let projection_scope = record.projection_scope.unwrap();
        let source = record.original.clone();
        let space = record.space.ok_or_else(stale)?;
        let participation = app
            .authorization
            .borrow()
            .consent
            .participation()
            .ok_or_else(stale)?;
        // 已 accepted 只读重建仍核原目标/Grant，但 NEW 策略不是撤销既有远端关联。
        let consent = if existing {
            None
        } else {
            Some(
                app.authorization
                    .borrow()
                    .consent
                    .stamp()
                    .ok_or_else(stale)?,
            )
        };
        let doc = app.documents.first().ok_or_else(stale)?;
        let frame = placement::bounds(doc.window.0, space)?;
        let guard = restore_batch::guard::Guard::capture(app)?;
        let scope = Self {
            source,
            space,
            frame,
            projection_scope,
            guard,
            consent,
            participation,
        };
        if !scope.matches(app) {
            return Err(stale());
        }
        Ok(scope)
    }
    pub(super) unsafe fn matches(&self, app: &Application) -> bool {
        app.authorization
            .borrow()
            .consent
            .participates(&self.participation)
            && self
                .consent
                .as_ref()
                .is_none_or(|stamp| app.authorization.borrow().consent.matches(stamp))
            && app.projection_checkpoint_available()
            && app.save_job.is_none()
            && app.restore_job.is_none()
            && app.restored_session.as_ref().is_some_and(|record| {
                record.original.same_storage(&self.source)
                    && record.projection_scope == Some(self.projection_scope)
                    && restore_batch::append::same_space(record.space, self.space)
            })
            && self.guard.matches(app)
            && app.documents.first().is_some_and(|doc| {
                placement::bounds(doc.window.0, self.space).is_ok_and(|bounds| bounds == self.frame)
            })
    }
}
pub(super) fn stale() -> Error {
    Error::new(
        E_ABORT,
        "Projection receive destination changed or unavailable",
    )
}
