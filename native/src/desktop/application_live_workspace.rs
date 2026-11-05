//! Live 与静态文档共享 graph 身份；生命周期收集按实际 owner，而不是仅按静态窗口。
use super::*;
impl Application {
    pub(in crate::desktop::application) fn admit_live(
        &mut self,
        mut session: Session,
        visible: bool,
    ) -> Result<()> {
        if self.live_sessions.len() >= MAX_SESSIONS
            || !fits_wait_budget(
                self.live_sessions.len() + 1,
                self.surface_windows.wait_budget(),
            )
        {
            return Err(Error::new(E_OUTOFMEMORY, "Live session limit reached"));
        }
        self.live_sessions
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate Live owner"))?;
        session.bind_workspace(&self.workspace);
        if !session.belongs_to(&self.workspace) || !session.is_alive() {
            return Err(Error::new(
                E_ABORT,
                "Live owner is closed or belongs to another workspace",
            ));
        }
        let unit = session.unit_id().clone();
        self.workspace
            .add_units([unit.clone()])
            .map_err(|error| Error::new(E_FAIL, format!("Cannot register Live unit: {error:?}")))?;
        if visible {
            if let Err(error) = session.show() {
                self.workspace.retain_units(|id| id != &unit);
                return Err(error);
            }
        }
        self.live_sessions.push(session);
        Ok(())
    }
    pub(in crate::desktop::application) fn retain_workspace_owners(&mut self) {
        let live: Vec<_> = self
            .live_sessions
            .iter()
            .filter(|session| session.belongs_to(&self.workspace) && session.is_alive())
            .map(|session| session.unit_id())
            .collect();
        self.workspace.retain_units(|id| {
            live.contains(&id)
                || self
                    .documents
                    .iter()
                    .any(|document| document.state.unit_id == *id && unsafe { document.is_alive() })
        });
        self.executions
            .retain_units(|id| self.workspace.contains(id));
        self.execution_jobs
            .retain_units(|id| self.workspace.contains(id));
    }
}
