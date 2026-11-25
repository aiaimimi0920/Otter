//! Owner of workspace identities and directed links; holds no HWNDs, pixels, or GPU resources and executes no nodes.
use crate::workspace_id::UnitId;
use std::{collections::HashMap, sync::Arc};
/// Immutable token for object/link topology; grouping and display changes do not invalidate the execution graph.
#[derive(Clone)]
pub struct TopologyStamp(Arc<()>);
/// Workspace-instance token; ordinary edits preserve it, while full restoration/replacement must create a new identity.
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
pub mod connect;
#[path = "workspace_groups.rs"]
pub mod groups;
#[path = "workspace_ports.rs"]
pub mod ports;
pub use groups::{Group, GroupStamp};
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
    memberships: HashMap<UnitId, UnitId>,
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
            memberships: HashMap::new(),
        }
    }
    pub fn units(&self) -> &[UnitId] {
        &self.units
    }
    pub fn identity_stamp(&self) -> IdentityStamp {
        IdentityStamp(self.identity.clone())
    }
    /// Replace a retired host while preserving other owners (including Live), groups, and unrelated links.
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
    /// Reverse-reachable set including the target; retain nodes currently stopping relays to preserve dependencies for future route changes.
    /// Borrow only identities and visit at most MAX_UNITS nodes; imported cycles and repeated multiport edges are not enqueued twice.
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
    /// Commit only after whole-batch validation/reservation succeeds; duplicate identities or budget failure leave no partial objects.
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
    /// Object release cascades to related edges; directed cycles are allowed, and executors own runtime cycle policy.
    pub fn retain_units(&mut self, mut keep: impl FnMut(&UnitId) -> bool) {
        let before = self.units.len();
        self.units.retain(|id| keep(id));
        // All relation admission paths require existing units. No deletion means no cascade.
        if before == self.units.len() {
            return;
        }
        let styles = self.port_visibility.len();
        self.port_visibility
            .retain(|(id, _)| self.units.contains(id));
        if styles != self.port_visibility.len() {
            self.appearance = Arc::new(());
        }
        self.topology = Arc::new(());
        self.memberships.retain(|unit, _| self.units.contains(unit));
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
