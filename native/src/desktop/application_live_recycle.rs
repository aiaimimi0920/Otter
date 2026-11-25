//! Live close-and-recycle: reuse the save slot and freeze policy; publish only if the candidate remains valid and window destruction succeeds.
use super::*;
impl Application {
    pub(super) unsafe fn process_live_recycle(&mut self) -> Result<()> {
        if self.modals.blocked()
            || self.automatic_save_pending()
            || self.live_freeze_job.is_some()
            || self
                .save_job
                .as_ref()
                .is_some_and(|(_, job)| job.recycle_unit().is_some())
        {
            return Ok(());
        }
        let Some(unit) = self.live_sessions.iter().find_map(|session| {
            session
                .take_recycle_request()
                .then(|| session.unit_id().clone())
        }) else {
            return Ok(());
        };
        if self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_requested.get()
            || !self.live_sessions.iter().any(|session| {
                session.unit_id() == &unit
                    && session.is_alive()
                    && session.belongs_to(&self.workspace)
            })
        {
            return Err(Error::new(
                E_ABORT,
                "Live recycle origin changed or workspace busy",
            ));
        }
        let (receipt, data) = self.capture_frozen_snapshot(&unit)?;
        let request = self.new_recycle_request(unit)?;
        let job =
            save_snapshot::job::Job::start_recycle(data, request, self.execution_wake.callback())
                .map_err(|error| Error::new(E_FAIL, error.to_string()))?;
        self.save_job = Some((receipt, job));
        Ok(())
    }
    pub(in crate::desktop::application) unsafe fn commit_live_recycle(
        &mut self,
        unit: &crate::workspace_id::UnitId,
        receipt: &save_snapshot::Receipt,
        source: crate::session_source::Source,
    ) -> Result<()> {
        if !receipt.matches(self) {
            return Err(Error::new(E_ABORT, "Live recycle changed before close"));
        }
        let session = self
            .live_sessions
            .iter_mut()
            .find(|session| session.unit_id() == unit)
            .ok_or_else(|| Error::new(E_ABORT, "Live recycle requester closed"))?;
        session.close_for_recycle()?;
        self.retain_workspace_owners();
        // Perform no further fallible preparation. Preserve the pre-save disk baseline; the library persists only when the session is subsequently saved.
        self.restored_session
            .as_mut()
            .expect("verified recycle source")
            .edit_original(source);
        self.saved_state = None;
        self.sync_frozen_menu();
        Ok(())
    }
}
