//! 只从默认工作区真实提交取得发送 scope；导入、暂存、手动另存不借旧 canonical 凭证。
use super::*;
impl Owner {
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
    /// 只读已提交的精确 sender 包含关系；scope/相同 UnitId/手动文件均不能借旧 canonical。
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
        // scope getter 同时核当前 Record/Workspace/真实 canonical Source；intent 必须属同一逻辑身份。
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
