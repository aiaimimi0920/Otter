//! Live 转静态提交：读回期间保留原会话，静态文档准备/登记成功后才停止源。
use super::*;
#[path = "live_freeze_job.rs"]
mod job;
#[cfg(test)]
#[path = "application_live_freeze_tests.rs"]
pub(in crate::desktop::application) mod tests;
pub(in crate::desktop::application) struct Pending {
    unit: crate::workspace_id::UnitId,
    source: crate::session_source::Source,
    job: job::Job,
}
impl Application {
    pub(super) unsafe fn process_live_freeze(&mut self, visible: bool) -> Result<()> {
        if self.modals.blocked() || self.automatic_save_pending() {
            return Ok(());
        }
        let busy = self.live_freeze_job.is_some();
        if let Some(result) = self
            .live_freeze_job
            .as_mut()
            .and_then(|pending| pending.job.poll())
        {
            let pending = self.live_freeze_job.take().expect("ready freeze owner");
            // 只在归还单槽时唤醒；其他 Live 的意图仍留在原窗口，不忙轮询。
            PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
            let pin = result.map_err(|error| Error::new(E_FAIL, error))?;
            self.commit_live_freeze(pending.unit, pending.source, pin, visible)?;
        }
        if busy
            || self
                .save_job
                .as_ref()
                .is_some_and(|(_, job)| job.recycle_unit().is_some())
        {
            return Ok(());
        }
        // destructive read 只能领取一个，且不能先领取再发现 worker 忙碌。
        let Some(unit) = self.live_sessions.iter().find_map(|session| {
            session
                .take_freeze_request()
                .then(|| session.unit_id().clone())
        }) else {
            return Ok(());
        };
        if self.save_job.is_some() || self.restore_job.is_some() || self.exit_requested.get() {
            return Err(Error::new(E_ABORT, "Workspace is busy during Live freeze"));
        }
        let session = self
            .live_sessions
            .iter()
            .find(|session| {
                session.unit_id() == &unit
                    && session.is_alive()
                    && session.belongs_to(&self.workspace)
            })
            .ok_or_else(|| Error::new(E_ABORT, "Live freeze requester closed"))?;
        let frame = session
            .snapshot_frame()?
            .ok_or_else(|| Error::new(E_ABORT, "Live has no current frame"))?;
        let source = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "Workspace source missing"))?
            .original
            .clone();
        let job = job::Job::start(frame, self.execution_wake.callback())
            .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        self.live_freeze_job = Some(Pending { unit, source, job });
        Ok(())
    }
    unsafe fn commit_live_freeze(
        &mut self,
        unit: crate::workspace_id::UnitId,
        source: crate::session_source::Source,
        pin: crate::desktop::open_image::PreparedDocument,
        visible: bool,
    ) -> Result<()> {
        if self.exit_requested.get()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || !self
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&source))
            || !self.live_sessions.iter().any(|session| {
                session.unit_id() == &unit
                    && session.is_alive()
                    && session.belongs_to(&self.workspace)
            })
        {
            return Err(Error::new(
                E_ABORT,
                "Live freeze origin or workspace changed",
            ));
        }
        let id = unit.clone();
        let size = [pin.image.width, pin.image.height];
        let session = self
            .live_sessions
            .iter()
            .find(|session| session.unit_id() == &unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live freeze origin closed"))?;
        let frame = session.freeze_rect(size)?;
        let opacity = session
            .opacity()
            .ok_or_else(|| Error::new(E_ABORT, "Live freeze opacity owner closed"))?;
        // 已登记的 Live 身份转交给 Document，不重复 add_units，也不先删除其连线/分组。
        if !self.workspace.contains(&id) || self.documents.iter().any(|doc| doc.state.unit_id == id)
        {
            return Err(Error::new(
                E_ABORT,
                "Live freeze identity is missing or already owned",
            ));
        }
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve frozen document"))?;
        self.capture_windows
            .borrow_mut()
            .try_reserve(2)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve frozen capture registry"))?;
        let topology = self.workspace.topology_stamp();
        let mut state = State::from_prepared(pin)?;
        {
            let mut canvas = state.canvas.borrow_mut();
            canvas.normal_opacity = opacity;
            canvas.window_opacity = opacity;
        }
        state.unit_id = id.clone();
        *state.execution_results.borrow_mut() =
            crate::desktop::loom_result_state::Results::new(id.clone());
        let document = self.prepare_window_at(state, visible, frame)?;
        if self.exit_requested.get()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || !topology.matches(&self.workspace)
            || !self.live_sessions.iter().any(|session| {
                session.unit_id() == &id
                    && session.is_alive()
                    && session.belongs_to(&self.workspace)
                    && session.opacity() == Some(opacity)
                    && session
                        .freeze_rect(size)
                        .is_ok_and(|current| current == frame)
            })
            || !self
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&source))
        {
            return Err(Error::new(
                E_ABORT,
                "Live freeze origin changed during window preparation",
            ));
        }
        self.documents.push(document);
        if let Some(session) = self
            .live_sessions
            .iter_mut()
            .find(|session| session.unit_id() == &unit)
        {
            session.stop();
        }
        self.sync_capture_windows();
        self.sync_groups();
        if visible {
            let document = self.documents.last().unwrap();
            if !document.state.group_hidden.get() {
                let _ = ShowWindow(document.window.0, SW_SHOWNORMAL);
                let _ = ShowWindow(document.panel.0, SW_SHOWNOACTIVATE);
            }
        }
        self.saved_state = None;
        Ok(())
    }
}
