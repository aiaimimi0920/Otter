//! 固定菜单来源的当前显示与工作区围栏；不在响应到达时重新挑选前台贴图。
use super::*;
use crate::{
    desktop::{display_snapshot, image::Image},
    editor::Editor,
    workspace_id::UnitId,
};
#[path = "application_projection_sender_content_stamp.rs"]
mod content;
#[path = "application_projection_sender_geometry.rs"]
mod geometry;
#[path = "application_projection_sender_snapshot.rs"]
mod snapshot;
pub(super) use content::ContentStamp;
pub(in crate::desktop::application) use snapshot::Snapshot;
pub(in crate::desktop::application) struct Scope {
    pub unit: UnitId,
    pub source: crate::session_source::Source,
    pub binding_source: crate::session_source::Source,
    pub scope: crate::session_projection_scope::Scope,
    pub workspace: crate::workspace::IdentityStamp,
    display: display_snapshot::Stamp,
    pixels: snapshot::Stamp,
    geometry: Option<geometry::Captured>,
}
impl Scope {
    pub(in crate::desktop::application) unsafe fn capture(
        app: &Application,
        unit: &UnitId,
    ) -> Result<(Self, Snapshot)> {
        if !app.projection_checkpoint_available()
            || app.save_job.is_some()
            || app.restore_job.is_some()
            || app.exit_requested.get()
            || app.modals.blocked()
            || app.display_composition.borrow().is_busy()
        {
            return Err(stale());
        }
        let scope = app.projection_sender_scope().ok_or_else(stale)?;
        let doc = app
            .documents
            .iter()
            .find(|doc| {
                doc.state.unit_id == *unit && app.workspace.contains(unit) && doc.is_alive()
            })
            .ok_or_else(stale)?;
        if !crate::desktop::application::restore_batch::guard::is_idle(app)
            || doc.state.execution_results.borrow().active().is_some()
        {
            return Err(stale());
        }
        let canvas = doc.state.canvas.borrow();
        let resolved = doc.state.resolved_display.borrow();
        let mut snapshot = Snapshot::capture(
            &canvas,
            resolved.as_ref(),
            if doc.state.is_art() {
                crate::projection_delivery::sender::Kind::Art
            } else {
                crate::projection_delivery::sender::Kind::Sticker
            },
            &app.documents,
        )
        .ok_or_else(stale)?;
        // 已有 full source/raster 的 editor 仍优先；可见 crop HWND 不能再次冒充完整源框。
        let geometry = if snapshot.editor.full_logical_frame().is_none()
            && snapshot.editor.layers().is_none()
        {
            Some(geometry::Captured::capture(
                doc,
                snapshot.kind,
                app.restored_session.as_ref().and_then(|r| r.space),
            )?)
        } else {
            None
        };
        snapshot.geometry = geometry.as_ref().map(|value| value.value);
        let captured = Self {
            unit: unit.clone(),
            // 新贴图尚未进入 Record.original；已证明的 canonical 保存才包含完整当前 Unit。
            // Record 原文另行保留作结果新鲜度/回滚围栏，不能混用两份 allocation。
            binding_source: app
                .workspace_checkpoint
                .as_ref()
                .and_then(|owner| owner.sender_source())
                .ok_or_else(stale)?
                .clone(),
            source: app
                .restored_session
                .as_ref()
                .ok_or_else(stale)?
                .original
                .clone(),
            scope,
            workspace: app.workspace.identity_stamp(),
            display: display_snapshot::Stamp::capture(&canvas, resolved.as_ref()),
            pixels: snapshot.stamp(),
            geometry,
        };
        Ok((captured, snapshot))
    }
    pub(in crate::desktop::application) unsafe fn matches(&self, app: &Application) -> bool {
        self.workspace.matches(&app.workspace)
            && display_matches(
                app,
                &self.unit,
                &self.display,
                &self.pixels,
                self.geometry.as_ref(),
            )
            && !app.exit_requested.get()
            && app.restore_job.is_none()
            && app.save_job.is_none()
            && app.projection_checkpoint_available()
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.source))
    }
    /// 初始化允许网络期间的矢量编辑；来源像素/几何与原canonical仍必须精确相同。
    pub(in crate::desktop::application) unsafe fn matches_basis(&self, app: &Application) -> bool {
        self.workspace.matches(&app.workspace)
            && !app.exit_requested.get()
            && app.restore_job.is_none()
            && app.save_job.is_none()
            && app.projection_checkpoint_available()
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.source))
            && app
                .documents
                .iter()
                .find(|doc| doc.state.unit_id == self.unit)
                .is_some_and(|doc| {
                    doc.is_alive()
                        && self.geometry.as_ref().is_none_or(|guard| {
                            guard.matches(doc, app.restored_session.as_ref().and_then(|r| r.space))
                        })
                        && self.pixels.matches(
                            &doc.state.canvas.borrow(),
                            doc.state.resolved_display.borrow().as_ref(),
                            &app.documents,
                        )
                })
    }
}
unsafe fn display_matches(
    app: &Application,
    unit: &UnitId,
    display: &display_snapshot::Stamp,
    pixels: &snapshot::Stamp,
    geometry: Option<&geometry::Captured>,
) -> bool {
    app.documents
        .iter()
        .find(|doc| doc.state.unit_id == *unit && app.workspace.contains(unit))
        .is_some_and(|doc| {
            doc.is_alive()
                && geometry.is_none_or(|guard| {
                    guard.matches(doc, app.restored_session.as_ref().and_then(|r| r.space))
                })
                && display.matches(
                    &doc.state.canvas.borrow(),
                    doc.state.resolved_display.borrow().as_ref(),
                )
                && pixels.matches(
                    &doc.state.canvas.borrow(),
                    doc.state.resolved_display.borrow().as_ref(),
                    &app.documents,
                )
        })
}
pub(in crate::desktop::application) fn stale() -> Error {
    Error::new(E_ABORT, "Projection sender source changed or unavailable")
}
