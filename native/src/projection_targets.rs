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
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireDirectory {
    targets: Vec<WireTarget>,
    // Missing fields support older Loom; explicit null is not a valid directory or group list.
    #[serde(default)]
    peer_directory: OptionField<PeerDirectory>,
    #[serde(default)]
    groups: OptionField<Vec<Group>>,
}
struct OptionField<T>(Option<T>);
impl<T> Default for OptionField<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for OptionField<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|value| Self(Some(value)))
    }
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
    pub fn parse(bytes: &[u8], origin: &str) -> Result<Self, Error> {
        if bytes.len() > MAX_JSON {
            return Err(Error::Limit);
        }
        let origin = envelope::canonical_origin(origin)?;
        let wire: WireDirectory = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        if wire.targets.len() > MAX_TARGETS
            || wire
                .groups
                .0
                .as_ref()
                .is_some_and(|groups| groups.len() > MAX_GROUPS)
        {
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
        for group in wire.groups.0.iter().flatten() {
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
        Ok(Self {
            origin,
            targets,
            status: wire
                .peer_directory
                .0
                .map_or(Status::Complete, |peer| peer.status),
            groups: wire.groups.0,
        })
    }
}
fn name_valid(value: &str, limit: usize, empty: bool) -> bool {
    (empty || !value.is_empty()) && value.len() <= limit && !value.chars().any(char::is_control)
}
#[cfg(test)]
#[path = "projection_targets_tests.rs"]
pub(crate) mod tests;
