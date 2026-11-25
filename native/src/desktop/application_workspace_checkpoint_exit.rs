//! Final time point for normal exit; preserve HWNDs until the original slot commits, and leave no latent exit after stale results/failures.
use super::*;

pub(super) struct Pending {
    intent: exit_intent::Intent,
    revision: u64,
}
impl Application {
    pub(in crate::desktop::application) fn checkpoint_writes_stopped(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.stopped)
    }
    pub(in crate::desktop::application) fn checkpoint_exit_waiting(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.exit.is_some())
    }
    pub(super) unsafe fn checkpoint_exit_matches(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .and_then(|owner| owner.exit.as_ref())
            .is_none_or(|pending| {
                restore_batch::guard::is_idle(self) && pending.intent.matches(self)
            })
    }
    pub(in crate::desktop::application) unsafe fn begin_checkpoint_exit(
        &mut self,
        intent: exit_intent::Intent,
    ) -> Result<bool> {
        let Some(owner) = self
            .workspace_checkpoint
            .as_ref()
            .filter(|owner| !owner.stopped)
        else {
            return Ok(false); // No profile/read-only recovery copies grant no new write authority; follow the explicit exit choice.
        };
        let revision = owner
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new(E_ABORT, "Checkpoint revision exhausted"))?;
        if !intent.matches(self) || !self.start_checkpoint_mode(true)? {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-exit-workspace-busy"),
            ));
        }
        self.workspace_checkpoint.as_mut().unwrap().exit = Some(Pending { intent, revision });
        Ok(true)
    }
    /// Some(false) also consumes this exit attempt; a later ordinary save must not revive old approval.
    pub(in crate::desktop::application) unsafe fn finish_checkpoint_exit(
        &mut self,
    ) -> Option<bool> {
        if !self.checkpoint_exit_waiting() {
            return None;
        }
        self.exit_requested.set(false); // Repeated exit clicks must not stale their own Receipt.
        if self.checkpoint_pending() {
            return Some(false);
        }
        let owner = self.workspace_checkpoint.as_mut().unwrap();
        let pending = owner.exit.take().unwrap();
        let committed = !owner.stopped && owner.revision == pending.revision;
        Some(
            committed
                && self.save_job.is_none()
                && restore_batch::guard::is_idle(self)
                && pending.intent.matches(self),
        )
    }
}
