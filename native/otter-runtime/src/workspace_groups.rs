//! Group metadata and single-group membership; deleting a group preserves objects, while closing it cascades to members and links.
use super::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    pub id: UnitId,
    pub name: String,
    pub hidden: bool,
    pub locked: bool,
}
/// Save group content rather than change counts: reordering the same members during assignment must not dirty the checkpoint.
pub struct GroupStamp {
    groups: Vec<Group>,
    memberships: HashMap<UnitId, UnitId>,
}
impl GroupStamp {
    pub fn matches(&self, workspace: &Workspace) -> bool {
        self.groups == workspace.groups && self.memberships == workspace.memberships
    }
}
impl Workspace {
    pub fn group_stamp(&self) -> GroupStamp {
        GroupStamp {
            groups: self.groups.clone(),
            memberships: self.memberships.clone(),
        }
    }
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
    pub fn group_of(&self, unit: &UnitId) -> Option<&Group> {
        let id = self.memberships.get(unit)?;
        self.groups.iter().find(|group| group.id == *id)
    }
    pub fn upsert_group(&mut self, group: Group) -> Result<(), Error> {
        if group.name.is_empty()
            || group.name.len() > 256
            || group.name.chars().any(char::is_control)
        {
            return Err(Error::InvalidName);
        }
        if let Some(existing) = self.groups.iter_mut().find(|item| item.id == group.id) {
            *existing = group;
        } else {
            if self.groups.len() >= MAX_UNITS {
                return Err(Error::Capacity);
            }
            self.groups.try_reserve(1).map_err(|_| Error::Allocation)?;
            self.groups.push(group);
        }
        Ok(())
    }
    /// Batch group assignment validates/reserves before modifying membership; each object belongs to at most one group.
    pub fn assign_group(&mut self, units: &[UnitId], group: Option<&UnitId>) -> Result<(), Error> {
        if units.len() > MAX_UNITS {
            return Err(Error::Capacity);
        }
        if units.iter().any(|id| !self.contains(id)) {
            return Err(Error::MissingUnit);
        }
        if group.is_some_and(|id| !self.groups.iter().any(|group| group.id == *id)) {
            return Err(Error::MissingGroup);
        }
        if let Some(group) = group {
            let new_members = units
                .iter()
                .filter(|unit| !self.memberships.contains_key(*unit))
                .count()
                .min(self.units.len() - self.memberships.len());
            self.memberships
                .try_reserve(new_members)
                .map_err(|_| Error::Allocation)?;
            for unit in units {
                match self.memberships.get_mut(unit) {
                    Some(current) if current != group => current.clone_from(group),
                    Some(_) => {}
                    None => {
                        self.memberships.insert(unit.clone(), group.clone());
                    }
                }
            }
        } else {
            for unit in units {
                self.memberships.remove(unit);
            }
        }
        Ok(())
    }
    pub fn remove_group(&mut self, id: &UnitId) -> bool {
        let before = self.groups.len();
        self.groups.retain(|group| group.id != *id);
        self.memberships.retain(|_, group| group != id);
        self.groups.len() != before
    }
    pub fn close_group(&mut self, id: &UnitId) -> Result<Vec<UnitId>, Error> {
        if !self.groups.iter().any(|group| group.id == *id) {
            return Err(Error::MissingGroup);
        }
        let mut members = Vec::new();
        members
            .try_reserve(self.memberships.len())
            .map_err(|_| Error::Allocation)?;
        members.extend(
            self.units
                .iter()
                // The baseline closes in object-collection order, not user group-selection order.
                .filter(|unit| self.memberships.get(*unit) == Some(id))
                .cloned(),
        );
        self.retain_units(|unit| !members.contains(unit));
        self.remove_group(id);
        Ok(members)
    }
}

#[cfg(test)]
#[path = "workspace_group_tests.rs"]
mod tests;
