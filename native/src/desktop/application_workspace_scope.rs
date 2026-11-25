//! Obtain send scope only from actual default-workspace commits; import, staging, and manual save-as must not borrow old canonical credentials.
use super::*;
impl Owner {
    pub(in crate::desktop::application) fn account_bindings(
        &self,
        workspace: &crate::workspace::Workspace,
    ) -> Vec<crate::projection_account::scene::Binding> {
        self.accounts
            .as_ref()
            .map(|index| {
                index
                    .bindings()
                    .iter()
                    .filter(|binding| self.includes_account(binding, workspace))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    pub(in crate::desktop::application) fn includes_account(
        &self,
        binding: &crate::projection_account::scene::Binding,
        workspace: &crate::workspace::Workspace,
    ) -> bool {
        !self.stopped
            && self
                .projection_workspace
                .as_ref()
                .is_some_and(|stamp| stamp.matches(workspace))
            && self.original.as_ref().is_some_and(|source| {
                self.accounts
                    .as_ref()
                    .is_some_and(|index| index.is_for(source) && index.includes(binding))
            })
            && self
                .projection_scope
                .as_ref()
                .is_some_and(|proof| proof.scope() == binding.scope())
    }
    pub(in crate::desktop::application) fn sender_source(
        &self,
    ) -> Option<&crate::session_source::Source> {
        self.original.as_ref()
    }
    pub(in crate::desktop::application) fn sender_index(
        &self,
    ) -> Option<&crate::projection_delivery::sender::binding::checkpoint::Index> {
        self.senders.as_ref()
    }
    pub(in crate::desktop::application) fn sender_deleted_bindings(
        &self,
        workspace: &crate::workspace::Workspace,
        unit: &str,
    ) -> Vec<crate::projection_delivery::sender::binding::Binding> {
        if !self
            .projection_workspace
            .as_ref()
            .is_some_and(|stamp| stamp.matches(workspace))
        {
            return Vec::new();
        }
        self.senders
            .as_ref()
            .map(|index| index.for_unit(unit).cloned().collect())
            .unwrap_or_default()
    }
}
impl Application {
    /// Read-only exact committed sender inclusion relationships; scope, identical UnitId, and manual files cannot borrow old canonical state.
    pub(in crate::desktop) fn projection_sender_inclusion(
        &self,
        expected: &crate::projection_delivery::sender::binding::Binding,
    ) -> bool {
        let Some(scope) = self.projection_sender_scope() else {
            return false;
        };
        if self.restore_job.is_some()
            || !self
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == expected.unit())
        {
            return false;
        }
        let owner = self.workspace_checkpoint.as_ref().unwrap();
        let Some(index) = owner.senders.as_ref() else {
            return false;
        };
        // The scope getter checks current Record/Workspace/actual canonical Source together; intent must belong to the same logical identity.
        index.is_for(owner.original.as_ref().unwrap())
            && index.includes(expected)
            && scope.to_string() == expected.scope()
    }
    pub(in crate::desktop) fn projection_sender_scope(
        &self,
    ) -> Option<crate::session_projection_scope::Scope> {
        let owner = self.workspace_checkpoint.as_ref()?;
        if owner.stopped || owner.exiting || owner.exit.is_some() {
            return None;
        }
        let record = self.restored_session.as_ref()?;
        let proof = owner.projection_scope.as_ref()?;
        if !owner
            .projection_workspace
            .as_ref()?
            .matches(&self.workspace)
            || !proof.is_for(owner.original.as_ref()?)
            || !owner.stamp.as_ref()?.1.same_storage(&record.original)
            || record.projection_scope != Some(proof.scope())
        {
            return None;
        }
        Some(proof.scope())
    }
}
