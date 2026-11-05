//! Switch 与共享编辑 FIFO 的接缝；不复用受 submitting 限制的 Click 单槽。
use super::*;
impl Mount {
    pub(super) fn sync_switches(
        &mut self,
        presentation: &Presentation,
        nodes: &BTreeMap<&str, &Node>,
        contracts: &BTreeMap<&str, [u8; 32]>,
    ) -> Result<()> {
        self.switches.retain(|id, field| {
            nodes.get(id.as_str()).is_some_and(|node| {
                node.kind() == Kind::Switch && contracts.get(id.as_str()) == Some(&field.contract)
            })
        });
        // 必须先清旧身份的 waiting/prepared，再创建同 ID 新会话；否则旧值会改投新 action。
        self.intents.retain(|id| {
            self.fields.contains_key(id)
                || self.switches.contains_key(id)
                || self.selects.contains_key(id)
                || self.sliders.contains_key(id)
        });
        for geometry in presentation.geometry().nodes() {
            let Some(node) = nodes
                .get(geometry.id())
                .filter(|node| node.kind() == Kind::Switch)
            else {
                continue;
            };
            if geometry.size().iter().any(|v| *v < 1.) {
                self.switches.remove(node.id());
                continue;
            }
            if !self.switches.contains_key(node.id()) {
                self.switches.insert(
                    node.id().to_owned(),
                    switch_session::Session::new(node, contracts[node.id()])?,
                );
            }
            let field = self.switches.get_mut(node.id()).unwrap();
            let before = field.value();
            field.synchronize(node)?;
            self.switch_changes |= before != field.value();
            if field.disabled {
                self.intents.retain(|id| id != node.id());
            }
        }
        Ok(())
    }
    pub fn switch_value(&self, id: &str) -> Option<bool> {
        self.switches.get(id).map(switch_session::Session::value)
    }
    /// 准入失败不修改可见值；在同一 UI owner 借用内完成 ticket/队列/本地值更新。
    pub fn toggle_switch(&mut self, id: &str) -> Result<bool> {
        if !self.editing {
            return Ok(false);
        }
        let Some(field) = self.switches.get_mut(id).filter(|field| !field.disabled) else {
            return Ok(false);
        };
        if !self.intents.can_push_change() {
            return Err(Error::new(E_FAIL, "Surface edit queue full"));
        }
        field.toggle()?;
        self.switch_changes = true;
        let pushed = self.intents.push(id, Phase::Change, field.snapshot());
        debug_assert!(pushed, "single UI owner reserved change capacity");
        Ok(true)
    }
    pub(in crate::desktop) fn take_switch_changes(&mut self) -> Vec<(String, bool, bool)> {
        if !std::mem::take(&mut self.switch_changes) {
            return Vec::new();
        }
        self.switches
            .iter_mut()
            .filter_map(|(id, field)| {
                field
                    .take_change()
                    .map(|(before, after)| (id.clone(), before, after))
            })
            .collect()
    }
}
