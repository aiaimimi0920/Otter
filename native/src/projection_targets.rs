//! 发送目标目录的原子校验；目录广告不是离线发送能力，也不构造任何创建请求。
use super::{envelope, Error, MAX_JSON};
use serde::Deserialize;
use std::collections::BTreeSet;
#[path = "projection_target_routes.rs"]
mod routes;
pub use routes::Route;
pub const MAX_TARGETS: usize = 64;
pub const MAX_GROUPS: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    Confirm,
    Auto,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Complete,
    Partial,
    Busy,
    Unavailable,
}
#[derive(Debug)]
pub struct Target {
    id: String,
    name: String,
    policy: Policy,
    route: Route,
}
impl Target {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn policy(&self) -> Policy {
        self.policy
    }
    pub fn route(&self) -> &Route {
        &self.route
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    group_id: String,
    name: String,
    target_ids: Vec<String>,
    unavailable_count: usize,
}
impl Group {
    pub fn id(&self) -> &str {
        &self.group_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn target_ids(&self) -> &[String] {
        &self.target_ids
    }
    pub fn unavailable_count(&self) -> usize {
        self.unavailable_count
    }
}
#[derive(Debug)]
pub struct Directory {
    origin: String,
    targets: Vec<Target>,
    status: Status,
    groups: Option<Vec<Group>>,
    users: Vec<User>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    user_id: String,
    issuer: String,
    account_id: String,
    name: String,
    target_ids: Vec<String>,
}
impl User {
    pub fn id(&self) -> &str {
        &self.user_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn target_ids(&self) -> &[String] {
        &self.target_ids
    }
}
#[derive(Deserialize)]
struct Users {
    users: Vec<User>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireDirectory {
    targets: Vec<WireTarget>,
    peer_directory: PeerDirectory,
    groups: Vec<Group>,
    friends: Users,
}
#[derive(Deserialize)]
struct PeerDirectory {
    status: Status,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireTarget {
    device_id: String,
    name: String,
    policy: Policy,
    #[serde(flatten)]
    route: routes::Wire,
}
impl Directory {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn groups(&self) -> Option<&[Group]> {
        self.groups.as_deref()
    }
    pub fn users(&self) -> &[User] {
        &self.users
    }
    pub fn parse(bytes: &[u8], origin: &str) -> Result<Self, Error> {
        if bytes.len() > MAX_JSON {
            return Err(Error::Limit);
        }
        let origin = envelope::canonical_origin(origin)?;
        let wire: WireDirectory = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        if wire.targets.len() > MAX_TARGETS || wire.groups.len() > MAX_GROUPS {
            return Err(Error::Limit);
        }
        let mut ids = BTreeSet::new();
        let mut targets = Vec::with_capacity(wire.targets.len());
        for target in wire.targets {
            if !envelope::identifier(&target.device_id) || !name_valid(&target.name, 1024, true) {
                return Err(Error::Entry);
            }
            if !ids.insert(target.device_id.clone()) {
                return Err(Error::Duplicate);
            }
            targets.push(Target {
                route: target.route.validate(&target.device_id)?,
                id: target.device_id,
                name: target.name,
                policy: target.policy,
            });
        }
        let mut groups = BTreeSet::new();
        for group in &wire.groups {
            if !envelope::identifier(&group.group_id)
                || !name_valid(&group.name, 128, false)
                || group.target_ids.len() > MAX_TARGETS
                || group.unavailable_count > MAX_TARGETS - group.target_ids.len()
                || group.target_ids.iter().any(|id| !ids.contains(id))
            {
                return Err(Error::Entry);
            }
            if !groups.insert(&group.group_id)
                || group.target_ids.iter().collect::<BTreeSet<_>>().len() != group.target_ids.len()
            {
                return Err(Error::Duplicate);
            }
        }
        let users = wire.friends.users;
        let mut user_ids = BTreeSet::new();
        if users.len() > 34 {
            return Err(Error::Limit);
        }
        for user in &users {
            if !envelope::identifier(&user.user_id)
                || !user_ids.insert(&user.user_id)
                || !name_valid(&user.name, 256, false)
                || !name_valid(&user.account_id, 256, false)
                || envelope::canonical_origin(&user.issuer).as_ref() != Ok(&user.issuer)
                || user.target_ids.len() > MAX_TARGETS
                || user.target_ids.iter().any(|id| !ids.contains(id))
                || user.target_ids.iter().collect::<BTreeSet<_>>().len() != user.target_ids.len()
            {
                return Err(Error::Entry);
            }
        }
        Ok(Self {
            origin,
            targets,
            status: wire.peer_directory.status,
            groups: Some(wire.groups),
            users,
        })
    }
}
fn name_valid(value: &str, limit: usize, empty: bool) -> bool {
    (empty || !value.is_empty()) && value.len() <= limit && !value.chars().any(char::is_control)
}
#[cfg(test)]
#[path = "projection_targets_tests.rs"]
pub(crate) mod tests;
