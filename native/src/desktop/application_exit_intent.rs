//! 一次性退出批准；只保持久模型/来源/Live 窗口状态，不保帧或持续捕获权限。
use super::*;
use crate::{session_source::Source, workspace_id::UnitId};

pub(super) struct Intent {
    model: save_state::Stamp,
    source: Option<Source>,
    live: Vec<(UnitId, RECT, f32)>,
}
impl Intent {
    pub(super) unsafe fn capture(app: &Application) -> Result<Self> {
        let live = app
            .live_sessions
            .iter()
            .filter(|session| session.belongs_to(&app.workspace))
            .map(|session| {
                let (rect, opacity) = session
                    .persistence_state()
                    .ok_or_else(|| Error::new(E_ABORT, "Cannot capture closing Live window"))?;
                Ok((session.unit_id().clone(), rect, opacity))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            model: save_state::Stamp::capture(app)?,
            source: app
                .restored_session
                .as_ref()
                .map(|record| record.original.clone()),
            live,
        })
    }
    pub(super) unsafe fn matches(&self, app: &Application) -> bool {
        // 无 profile 的明确放弃沿原路径清理执行；完整快照空闲只归 checkpoint 边界。
        !app.modals.blocked()
            && app.chrome.idle()
            && !app.exit_requested.get()
            && !app.naming_autosave_blocks_exit()
            && app.restore_job.is_none()
            && app.live_freeze_job.is_none()
            && !app.projection_receive.preparing()
            && app.open_after_save.is_none()
            && app.documents.iter().all(|doc| {
                !doc.state.new_window.get()
                    && doc.state.art_request.borrow().is_none()
                    && !doc.state.save_session_requested.get()
                    && !doc.state.save_existing_requested.get()
                    && !doc.state.open_session_requested.get()
            })
            && match (&self.source, &app.restored_session) {
                (Some(source), Some(record)) => source.same_storage(&record.original),
                (None, None) => true,
                _ => false,
            }
            && self.model.matches_model(app)
            && self.live.len()
                == app
                    .live_sessions
                    .iter()
                    .filter(|s| s.belongs_to(&app.workspace))
                    .count()
            && self.live.iter().all(|(id, rect, opacity)| {
                app.live_sessions.iter().any(|session| {
                    session.belongs_to(&app.workspace)
                        && session.unit_id() == id
                        && session.exit_idle()
                        && session.persistence_state() == Some((*rect, *opacity))
                })
            })
    }
}
