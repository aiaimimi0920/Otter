//! 分组元数据与单组成员关系；删除组不删除对象，关闭组才级联删除成员和链接。
use super::*;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Group {
    pub id: UnitId,
    pub name: String,
    pub hidden: bool,
    pub locked: bool,
}
impl Workspace {
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }
    pub fn group_of(&self, unit: &UnitId) -> Option<&Group> {
        let (_, id) = self.memberships.iter().find(|(member, _)| member == unit)?;
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
    /// 批量赋组先校验所有对象和目标组，再交换候选；重复选择不会产生重复成员。
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
        let mut next = Vec::new();
        next.try_reserve(self.units.len())
            .map_err(|_| Error::Allocation)?;
        next.extend(
            self.memberships
                .iter()
                .filter(|(unit, _)| !units.contains(unit))
                .cloned(),
        );
        if let Some(group) = group {
            for unit in units {
                if !next.iter().any(|(member, _)| member == unit) {
                    next.push((unit.clone(), group.clone()));
                }
            }
        }
        self.memberships = next;
        Ok(())
    }
    pub fn remove_group(&mut self, id: &UnitId) -> bool {
        let before = self.groups.len();
        self.groups.retain(|group| group.id != *id);
        self.memberships.retain(|(_, group)| group != id);
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
                // 主干按对象集合顺序关闭，不按用户分组选择顺序关闭。
                .filter(|unit| {
                    self.memberships
                        .iter()
                        .any(|(member, group)| member == *unit && group == id)
                })
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
