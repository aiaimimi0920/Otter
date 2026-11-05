//! 工作区身份和有向链接的所有者；不持有 HWND、像素或 GPU 资源，不执行节点。
use crate::workspace_id::UnitId;
use std::sync::Arc;
/// 对象与连线拓扑的不可变凭证；分组和显示变化不使执行图失效。
#[derive(Clone)]
pub struct TopologyStamp(Arc<()>);
/// 工作区实例凭证；普通编辑不改变，整体恢复/替换必须产生新身份。
#[derive(Clone)]
pub struct IdentityStamp(Arc<()>);
impl IdentityStamp {
    pub fn matches(&self, workspace: &Workspace) -> bool {
        Arc::ptr_eq(&self.0, &workspace.identity)
    }
}
impl TopologyStamp {
    pub fn matches(&self, workspace: &Workspace) -> bool {
        Arc::ptr_eq(&self.0, &workspace.topology)
    }
}
#[path = "workspace_connect.rs"]
mod connect;
#[path = "workspace_groups.rs"]
mod groups;
#[path = "workspace_ports.rs"]
pub mod ports;
pub use groups::Group;
pub const MAX_UNITS: usize = 128;
pub const MAX_LINKS: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    pub id: UnitId,
    pub from_unit: UnitId,
    pub from_port: String,
    pub to_unit: UnitId,
    pub to_port: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Capacity,
    DuplicateId,
    MissingUnit,
    InvalidPort,
    Allocation,
    MissingGroup,
    InvalidName,
    SelfConnection,
    Cycle,
}

#[derive(Clone, Default)]
pub struct Workspace {
    identity: Arc<()>,
    topology: Arc<()>,
    appearance: Arc<()>,
    port_visibility: Vec<(UnitId, ports::Visibility)>,
    units: Vec<UnitId>,
    links: Vec<Link>,
    groups: Vec<Group>,
    memberships: Vec<(UnitId, UnitId)>,
}
impl Workspace {
    pub fn with_initial(id: UnitId) -> Self {
        Self {
            identity: Arc::new(()),
            topology: Arc::new(()),
            appearance: Arc::new(()),
            port_visibility: Vec::new(),
            units: vec![id],
            links: Vec::new(),
            groups: Vec::new(),
            memberships: Vec::new(),
        }
    }
    pub fn units(&self) -> &[UnitId] {
        &self.units
    }
    pub fn identity_stamp(&self) -> IdentityStamp {
        IdentityStamp(self.identity.clone())
    }
    /// 替换一个已退役宿主；保留其他 owner（包括 Live）、分组及无关连线。
    pub fn replacement_unit(&self, old: &UnitId, id: UnitId) -> Result<Self, Error> {
        if !self.contains(old) {
            return Err(Error::MissingUnit);
        }
        if self.contains(&id) {
            return Err(Error::DuplicateId);
        }
        let mut next = self.clone();
        next.retain_units(|unit| unit != old);
        next.add_units([id])?;
        Ok(next)
    }
    pub fn topology_stamp(&self) -> TopologyStamp {
        TopologyStamp(self.topology.clone())
    }
    pub fn links(&self) -> &[Link] {
        &self.links
    }
    pub fn contains(&self, id: &UnitId) -> bool {
        self.units.contains(id)
    }
    /// 包含目标的反向可达集合；不剪掉当前停止中继的节点，以保留未来改路的依赖。
    /// 只借用身份，最多访问 MAX_UNITS 个节点；导入环及多端口重复边不会重复入队。
    pub fn upstream_units(&self, target: &UnitId) -> Result<Vec<&UnitId>, Error> {
        let target = self
            .units
            .iter()
            .find(|unit| *unit == target)
            .ok_or(Error::MissingUnit)?;
        let mut found = Vec::new();
        found
            .try_reserve(self.units.len())
            .map_err(|_| Error::Allocation)?;
        found.push(target);
        let mut next = 0;
        while next < found.len() {
            let current = found[next];
            for link in self.links.iter().filter(|link| &link.to_unit == current) {
                if !found.contains(&&link.from_unit) {
                    found.push(&link.from_unit);
                }
            }
            next += 1;
        }
        Ok(found)
    }
    /// 全批验证/预留成功才提交，重复身份或预算失败不留下部分对象。
    pub fn add_units(&mut self, ids: impl IntoIterator<Item = UnitId>) -> Result<(), Error> {
        let mut added = Vec::new();
        for id in ids {
            if self.units.len() + added.len() >= MAX_UNITS {
                return Err(Error::Capacity);
            }
            if self.contains(&id) || added.contains(&id) {
                return Err(Error::DuplicateId);
            }
            added.try_reserve(1).map_err(|_| Error::Allocation)?;
            added.push(id);
        }
        self.units
            .try_reserve(added.len())
            .map_err(|_| Error::Allocation)?;
        if !added.is_empty() {
            self.topology = Arc::new(());
            self.units.extend(added);
        }
        Ok(())
    }
    pub fn add_link(&mut self, link: Link) -> Result<(), Error> {
        if self.links.len() >= MAX_LINKS {
            return Err(Error::Capacity);
        }
        if self.links.iter().any(|item| item.id == link.id) {
            return Err(Error::DuplicateId);
        }
        if !self.contains(&link.from_unit) || !self.contains(&link.to_unit) {
            return Err(Error::MissingUnit);
        }
        if !valid_port(&link.from_port) || !valid_port(&link.to_port) {
            return Err(Error::InvalidPort);
        }
        self.links.try_reserve(1).map_err(|_| Error::Allocation)?;
        self.topology = Arc::new(());
        self.links.push(link);
        Ok(())
    }
    pub fn remove_link(&mut self, id: &UnitId) -> bool {
        let before = self.links.len();
        self.links.retain(|link| link.id != *id);
        let changed = before != self.links.len();
        if changed {
            self.topology = Arc::new(());
        }
        changed
    }
    /// 对象释放时级联移除相关边；不限制有向环，执行期的循环策略由执行器负责。
    pub fn retain_units(&mut self, mut keep: impl FnMut(&UnitId) -> bool) {
        let before = self.units.len();
        self.units.retain(|id| keep(id));
        let styles = self.port_visibility.len();
        self.port_visibility
            .retain(|(id, _)| self.units.contains(id));
        if styles != self.port_visibility.len() {
            self.appearance = Arc::new(());
        }
        if before != self.units.len() {
            self.topology = Arc::new(());
        }
        self.memberships
            .retain(|(unit, _)| self.units.contains(unit));
        self.links.retain(|link| {
            self.units.contains(&link.from_unit) && self.units.contains(&link.to_unit)
        });
    }
    pub fn clear(&mut self) {
        if !self.units.is_empty() || !self.links.is_empty() {
            self.topology = Arc::new(());
        }
        self.units.clear();
        if !self.port_visibility.is_empty() {
            self.appearance = Arc::new(());
            self.port_visibility.clear();
        }
        self.links.clear();
        self.groups.clear();
        self.memberships.clear();
    }
}
fn valid_port(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "workspace_topology_tests.rs"]
mod topology_tests;
#[cfg(test)]
#[path = "workspace_upstream_tests.rs"]
mod upstream_tests;
