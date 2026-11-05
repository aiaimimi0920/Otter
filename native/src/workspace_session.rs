//! 主干会话的拓扑适配；字段与 SessionSticker/Link/Group 对齐，不负责图片/编辑器或磁盘写入。
use crate::{
    workspace::{Error as ModelError, Group, Link, Workspace, MAX_LINKS, MAX_UNITS},
    workspace_id::UnitId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sticker {
    pub id: String,
    #[serde(default)]
    pub group_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionLink {
    pub id: String,
    pub from_unit_id: String,
    pub from_port_id: String,
    pub to_unit_id: String,
    pub to_port_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub locked: bool,
}
/// 仅拓扑切片，不可把它序列化后覆盖完整旧会话，否则会丢失图片/标注等字段。
#[derive(Debug, Clone, PartialEq)]
pub struct Topology {
    pub stickers: Vec<Sticker>,
    pub links: Vec<SessionLink>,
    pub groups: Vec<SessionGroup>,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Identity,
    DuplicateGroup,
    Model(ModelError),
}
impl From<ModelError> for Error {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}
fn id(value: &str) -> Result<UnitId, Error> {
    UnitId::parse(value).ok_or(Error::Identity)
}

impl Topology {
    pub fn capture(workspace: &Workspace) -> Self {
        Self {
            stickers: workspace
                .units()
                .iter()
                .map(|unit| Sticker {
                    id: unit.as_str().into(),
                    group_id: workspace
                        .group_of(unit)
                        .map(|group| group.id.as_str().into()),
                })
                .collect(),
            links: workspace
                .links()
                .iter()
                .map(|link| SessionLink {
                    id: link.id.as_str().into(),
                    from_unit_id: link.from_unit.as_str().into(),
                    from_port_id: link.from_port.clone(),
                    to_unit_id: link.to_unit.as_str().into(),
                    to_port_id: link.to_port.clone(),
                })
                .collect(),
            groups: workspace
                .groups()
                .iter()
                .map(|group| SessionGroup {
                    id: group.id.as_str().into(),
                    name: group.name.clone(),
                    hidden: group.hidden,
                    locked: group.locked,
                })
                .collect(),
        }
    }
    /// 先完整构造独立候选。调用者还需验证其他会话资源，全部成功后才交换当前工作区。
    pub fn restore(&self) -> Result<Workspace, Error> {
        if self.stickers.len() > MAX_UNITS
            || self.groups.len() > MAX_UNITS
            || self.links.len() > MAX_LINKS
        {
            return Err(ModelError::Capacity.into());
        }
        let mut workspace = Workspace::default();
        let units = self
            .stickers
            .iter()
            .map(|sticker| id(&sticker.id))
            .collect::<Result<Vec<_>, _>>()?;
        workspace.add_units(units)?;
        for group in &self.groups {
            if group.name.is_empty()
                || group.name.len() > 256
                || group.name.chars().any(char::is_control)
            {
                return Err(ModelError::InvalidName.into());
            }
            let group_id = id(&group.id)?;
            if workspace.groups().iter().any(|value| value.id == group_id) {
                return Err(Error::DuplicateGroup);
            }
            workspace.upsert_group(Group {
                id: group_id,
                name: group.name.clone(),
                hidden: group.hidden,
                locked: group.locked,
            })?;
        }
        for sticker in &self.stickers {
            if let Some(group) = &sticker.group_id {
                workspace.assign_group(&[id(&sticker.id)?], Some(&id(group)?))?;
            }
        }
        for link in &self.links {
            if [&link.from_port_id, &link.to_port_id].iter().any(|port| {
                port.is_empty() || port.len() > 256 || port.chars().any(char::is_control)
            }) {
                return Err(ModelError::InvalidPort.into());
            }
            // 恢复不是交互 connect：保留旧会话的边顺序/身份，不用单输入替换规则改写数据。
            workspace.add_link(Link {
                id: id(&link.id)?,
                from_unit: id(&link.from_unit_id)?,
                from_port: link.from_port_id.clone(),
                to_unit: id(&link.to_unit_id)?,
                to_port: link.to_port_id.clone(),
            })?;
        }
        Ok(workspace)
    }
}

#[cfg(test)]
#[path = "workspace_session_tests.rs"]
mod tests;
