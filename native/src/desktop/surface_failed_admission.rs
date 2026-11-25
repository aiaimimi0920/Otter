//! Lifecycle owner for attachments without windows; retain original mount evidence separately from current deactivation state.
use crate::{
    surface_activation::Activation,
    surface_attach_runtime,
    surface_lifecycle_owner::Owner,
    surface_lifecycle_runtime::{Outcome, StartError},
    surface_live_binding::Lifecycle,
};

pub struct FailedAdmission {
    pub outcome: surface_attach_runtime::Outcome,
    pub reason: String,
    lifecycle: Option<Activation>,
    rejected: Option<Box<Owner>>,
}
impl FailedAdmission {
    pub(super) fn append_recovery_records(
        &self,
        records: &mut Vec<crate::surface_recovery_record::Record>,
    ) {
        use crate::surface_recovery_record::Record;
        let binding = self
            .lifecycle
            .as_ref()
            .map(Activation::binding)
            .or(self.outcome.binding.as_ref());
        records.push(match binding {
            Some(binding) => Record::from_binding(
                binding,
                Some(self.outcome.request.art()),
                self.outcome.uncertain
                    || self.pending()
                    || self.lifecycle().is_some_and(Owner::uncertain),
                self.pending(),
            ),
            None => Record::from_request(
                &self.outcome.origin,
                &self.outcome.device,
                &self.outcome.request,
                self.outcome.uncertain,
            ),
        });
        if let Some(owner) = &self.rejected {
            records.push(Record::from_binding(
                owner.binding(),
                None,
                owner.uncertain(),
                false,
            ));
        }
    }
    pub(super) fn new(
        outcome: surface_attach_runtime::Outcome,
        reason: String,
        duplicate: bool,
    ) -> Self {
        // Duplicate evidence creates no second owner; unconfirmed or mismatched identities also forbid guessing which object to deactivate.
        let lifecycle = outcome
            .binding
            .as_ref()
            .filter(|binding| {
                !duplicate
                    && !outcome.uncertain
                    && binding.origin() == outcome.origin
                    && binding.device() == outcome.device
                    && binding.node() == outcome.request.node()
            })
            .map(|binding| Activation::new(Owner::new(binding.clone())));
        Self {
            outcome,
            reason,
            lifecycle,
            rejected: None,
        }
    }
    pub fn lifecycle(&self) -> Option<&Owner> {
        self.lifecycle.as_ref().and_then(Activation::owner)
    }
    pub(super) fn pending(&self) -> bool {
        self.lifecycle
            .as_ref()
            .is_some_and(|lifecycle| lifecycle.owner().is_none())
    }
    pub(super) fn settled(&self) -> bool {
        self.terminal_owner().is_some()
    }
    pub(super) fn terminal_owner(&self) -> Option<&Owner> {
        if self.rejected.is_some() {
            return None;
        }
        self.lifecycle().filter(|owner| {
            !owner.uncertain()
                && matches!(
                    owner.binding().lifecycle(),
                    Lifecycle::Inactive | Lifecycle::Disposed
                )
        })
    }
    pub(in crate::desktop) fn begin_inactive(&mut self) -> Option<Owner> {
        self.lifecycle.as_mut()?.take_inactive()
    }
    pub(in crate::desktop) fn failed_start(&mut self, error: StartError) {
        self.reason = format!("附件停用未启动：{}", error.error);
        if let Some(lifecycle) = &mut self.lifecycle {
            if let Err(owner) = lifecycle.restore(*error.owner) {
                self.rejected = Some(owner);
            }
        } else {
            self.rejected = Some(error.owner);
        }
    }
    pub(super) fn accept(&mut self, outcome: Outcome) -> Result<(), Box<Outcome>> {
        if matches!(&outcome.result, Ok(Some(_))) {
            return Err(Box::new(outcome));
        }
        let Some(lifecycle) = &mut self.lifecycle else {
            return Err(Box::new(outcome));
        };
        let Outcome { owner, result } = outcome;
        if let Err(owner) = lifecycle.restore(owner) {
            return Err(Box::new(Outcome {
                owner: *owner,
                result,
            }));
        }
        if let Err(error) = result {
            self.reason = format!("附件停用未完成：{error:?}");
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_failed_admission_tests.rs"]
mod tests;
