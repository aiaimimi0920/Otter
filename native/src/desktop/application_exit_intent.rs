//! One-shot exit approval; retain only persistent model/source/Live-window state, never frames or continuous-capture permission.
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
        // Explicit discard without a profile cleans up execution through the original path; full-snapshot idle requirements belong only to the checkpoint boundary.
        !app.modals.blocked()
            && app.chrome.idle()
            && !app.exit_requested.get()
            && !app.naming_autosave_blocks_exit()
            && !app.locale_blocks_exit()
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
