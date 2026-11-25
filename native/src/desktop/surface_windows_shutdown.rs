//! Surface terminal-state admission: normal reclamation/exit checks share one predicate, without retries or persistence claims.
use super::*;
use crate::{
    desktop::composition::relay::sessions::Sessions,
    surface_lifecycle_owner::Owner,
    surface_live_binding::Lifecycle,
    surface_recovery_record::{self, Record},
};

fn terminal_owner(window: &AuthorizedWindow) -> Option<&Owner> {
    if window.is_alive() || !window.javascript_finished() || window.rejected_owner().is_some() {
        return None;
    }
    window.lifecycle().filter(|owner| {
        !owner.uncertain()
            && matches!(
                owner.binding().lifecycle(),
                Lifecycle::Inactive | Lifecycle::Disposed
            )
    })
}

impl Windows {
    pub(crate) fn retired_records(&self) -> &[Record] {
        &self.retired
    }

    /// Create persistable prompts before releasing owners; over-budget results neither evict existing evidence nor write disks in the message pump.
    /// Return true only for the first retirement block, avoiding repeated notifications each round.
    pub(crate) fn retire_confirmed(&mut self) -> bool {
        if self.retirement_blocked {
            return false;
        }
        let mut index = 0;
        while let Some(window) = self.entries.get(index) {
            let Some(owner) = terminal_owner(window) else {
                index += 1;
                continue;
            };
            let record =
                Record::from_binding(owner.binding(), Some(window.request().art()), false, false);
            if !self.stage_retirement(record) {
                return true;
            }
            self.entries.remove(index);
        }
        let mut index = 0;
        while let Some(failure) = self.failures.get(index) {
            let Some(owner) = failure.terminal_owner() else {
                index += 1;
                continue;
            };
            let record = Record::from_binding(
                owner.binding(),
                Some(failure.outcome.request.art()),
                false,
                false,
            );
            if !self.stage_retirement(record) {
                return true;
            }
            self.failures.remove(index);
        }
        false
    }

    fn stage_retirement(&mut self, record: Record) -> bool {
        if self.retired.len() < surface_recovery_record::MAX_RECORDS
            && self.retired.try_reserve(1).is_ok()
        {
            self.retired.push(record);
            if surface_recovery_record::encode(&self.retired).is_ok() {
                return true;
            }
            self.retired.pop();
        }
        self.retirement_blocked = true;
        false
    }

    pub fn recovery_records(&self) -> Vec<Record> {
        let mut records = Vec::new();
        for window in &self.entries {
            let owner = window.lifecycle();
            records.push(Record::from_binding(
                owner.map_or_else(|| window.binding(), |owner| owner.binding()),
                Some(window.request().art()),
                owner.is_none_or(|owner| owner.uncertain()),
                owner.is_none(),
            ));
            if let Some(owner) = window.rejected_owner() {
                records.push(Record::from_binding(
                    owner.binding(),
                    None,
                    owner.uncertain(),
                    false,
                ));
            }
        }
        for failure in &self.failures {
            failure.append_recovery_records(&mut records);
        }
        records
    }
    /// Call after background reclamation; retain every failed/unknown record for upper-layer reporting or later persistence.
    pub fn settle_shutdown(&mut self, remote: &mut Sessions, now_ms: u64) -> Result<()> {
        let close_error = self.close_all().err();
        self.poll(now_ms)?;
        while remote.deliver_surface(|outcome| self.accept_activation(outcome, now_ms, false)) {}
        if let Some(error) = close_error {
            return Err(error);
        }
        let unresolved = remote.surface_cleanup_pending()
            || self.failures.iter().any(|failure| !failure.settled())
            || self
                .entries
                .iter()
                .any(|window| terminal_owner(window).is_none());
        if unresolved {
            Err(Error::new(
                E_FAIL,
                "Surface shutdown has unresolved attachment state",
            ))
        } else {
            Ok(())
        }
    }
}
