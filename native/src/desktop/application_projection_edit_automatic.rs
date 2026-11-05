//! 自动写入只接成功的原source probe/receiver read；不另建timer/队列，不重放未知pending。
use super::*;
use crate::workspace_id::UnitId;

#[derive(Default)]
pub(super) struct Blocked {
    workspace: Option<crate::workspace::IdentityStamp>,
    units: Vec<UnitId>,
}
impl Blocked {
    fn reconcile(&mut self, workspace: &crate::workspace::Workspace) {
        if !self
            .workspace
            .as_ref()
            .is_some_and(|stamp| stamp.matches(workspace))
        {
            self.workspace = Some(workspace.identity_stamp());
            self.units.clear();
        }
    }
    pub(super) fn block(&mut self, workspace: &crate::workspace::Workspace, unit: &UnitId) {
        self.reconcile(workspace);
        if !self.units.contains(unit) && self.units.len() < 64 {
            self.units.push(unit.clone());
        }
    }
    pub(super) fn confirmed(&mut self, unit: &UnitId) {
        self.units.retain(|blocked| blocked != unit);
    }
    fn permits(&mut self, workspace: &crate::workspace::Workspace, unit: &UnitId) -> bool {
        self.reconcile(workspace);
        self.units.len() < 64 && !self.units.contains(unit)
    }
}
#[cfg(test)]
impl Owner {
    pub(in crate::desktop::application) fn pause_automatic_for_test(
        &mut self,
        workspace: &crate::workspace::Workspace,
        unit: &UnitId,
    ) {
        self.1.block(workspace, unit);
    }
}
impl Application {
    pub(in crate::desktop::application) unsafe fn automatic_projection_edit(
        &mut self,
        unit: UnitId,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            Config,
            crate::device_authorization_config::Error,
        >,
    ) {
        if self.projection_edit.busy() || !self.projection_edit.1.permits(&self.workspace, &unit) {
            return;
        }
        let selection = self
            .documents
            .iter()
            .find(|doc| doc.is_alive() && doc.state.unit_id == unit)
            .and_then(|doc| {
                let slot = doc.state.projection_edit.borrow();
                let owner = slot.as_ref()?;
                let snapshot = owner.capture(&doc.state.canvas.borrow().editor);
                crate::projection_delivery_runtime::edit::automatic(&snapshot).then(|| Selection {
                    unit: unit.clone(),
                    stamp: owner.stamp(),
                })
            });
        if let Some(selection) = selection {
            // 单次自动触发至多32项；后续由原750ms读调度继续。失败/取消一直阻断到显式恢复成功。
            self.start_projection_edit(selection, config, 1, Action::Automatic);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_pause_is_bounded_deduplicated_and_only_confirmation_or_workspace_change_releases()
    {
        let workspace = crate::workspace::Workspace::default();
        let unit = |i| UnitId::parse(&format!("unit-{i}")).unwrap();
        let mut blocked = Blocked::default();
        assert!(blocked.permits(&workspace, &unit(0)));
        for _ in 0..2 {
            for i in 0..64 {
                blocked.block(&workspace, &unit(i));
            }
        }
        blocked.block(&workspace, &unit(64));
        assert_eq!(blocked.units.len(), 64);
        assert!(!blocked.permits(&workspace, &unit(64)));
        blocked.confirmed(&unit(0));
        assert!(!blocked.permits(&workspace, &unit(1)));
        assert!(blocked.permits(&workspace, &unit(0)));
        assert!(blocked.permits(&crate::workspace::Workspace::default(), &unit(1)));
        assert!(blocked.units.is_empty());
    }
}
