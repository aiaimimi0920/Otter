//! 同步候选提交：所有校验先于不可失败的源/Binding/Canvas 交换，不建窗或发 displayed。
use super::*;
impl Application {
    pub(super) unsafe fn publish_projection_sync(
        &mut self,
        pending: &Pending,
        update: Update,
    ) -> Result<()> {
        if !self.sync_pending_current(pending)
            || !update.index.includes(&update.binding)
            || !update.index.is_for(&update.source)
            || !update.binding.matches_ticket(&pending.ticket)
            || update.binding.revision() < pending.expected.revision()
        {
            return Err(stale());
        }
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == pending.ticket.unit())
            .ok_or_else(stale)?;
        let mut canvas = doc.state.canvas.borrow_mut();
        let edit = update
            .editing
            .map(|document| {
                use crate::projection_edit::{journal::Context, Route};
                crate::desktop::projection_edit_workspace::receiver::prepare(
                    doc.state.projection_edit.borrow().as_ref(),
                    document,
                    Context::new(
                        self.restored_session
                            .as_ref()
                            .ok_or_else(stale)?
                            .projection_scope
                            .ok_or_else(stale)?,
                        pending.ticket.unit(),
                        pending.ticket.origin(),
                        pending.ticket.device(),
                    )
                    .map_err(|_| stale())?,
                    pending.ticket.envelope().projection_id(),
                    if pending.ticket.invitation().peer_id().is_some() {
                        Route::Offline
                    } else {
                        Route::Shared
                    },
                    update.editing_frame.ok_or_else(stale)?,
                    &canvas.editor,
                )
            })
            .transpose()?;
        if edit.as_ref().is_some_and(|edit| !edit.changed)
            && update.image.is_none()
            && update.binding == pending.expected
        {
            return Ok(());
        }
        if let Some(edit) = edit {
            // 底图和矢量同一个Editor事务；任一预检拒绝都保留旧Editor/owner/source。
            let owner = edit.apply(
                &mut canvas.editor,
                update.image.as_ref().map(|image| image.base.clone()),
            )?;
            *doc.state.projection_edit.borrow_mut() = Some(owner);
        } else if let Some(image) = &update.image {
            if !canvas.editor.replace_source(image.base.clone()) {
                return Err(stale());
            }
        }
        if let Some(image) = update.image {
            canvas.sync_raster_source();
            canvas.minified_source = Some(crate::desktop::canvas::minified_source::Source::new(
                image.original,
                &canvas.editor,
            ));
            canvas.discard_device();
            // 原输出属于旧源；同时清运行时 owner，避免后续保存把旧端口写回原文。
            *doc.state.execution_results.borrow_mut() =
                crate::desktop::loom_result_state::Results::new(doc.state.unit_id.clone());
            *doc.state.resolved_display.borrow_mut() = None;
            doc.state
                .hit_region
                .borrow_mut()
                .replace_source(image.base, image.mask);
        }
        let edit_status = doc.state.projection_edit.borrow().as_ref().map(|owner| {
            let snapshot = owner.capture(&canvas.editor);
            if snapshot.conflict().is_some() {
                "协作冲突，保留本地修改"
            } else if snapshot
                .editor_view()
                .is_ok_and(|view| view.basis_objects() != &snapshot.document().live_objects())
            {
                "已接收协作，本地修改待提交"
            } else {
                "协作标注已同步"
            }
        });
        drop(canvas);
        let stopped = update.binding.stopped();
        if let Some(entry) = self
            .projection_receive
            .1
            .entries
            .iter_mut()
            .find(|entry| entry.ticket.same_record(&pending.ticket))
        {
            entry.binding = update.binding.clone();
        }
        let record = self.restored_session.as_mut().unwrap();
        record.edit_original(update.source);
        record.restored_projections = Some(update.index);
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        *self.display_composition.borrow_mut() = Default::default();
        self.authorization.borrow_mut().projection.status = if stopped {
            "投送关联已停止，保留最后内容"
        } else {
            edit_status.unwrap_or("投送已同步")
        }
        .into();
        // 只排本窗口重绘，不强制前台、不重置 frame/opacity/selection/viewport。
        let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(doc.window.0), None, false);
        Ok(())
    }
}
