//! 扩展目录信任边界：同主干预算、scope 归属和 when 条件；快照本身不授权资源读取。
use crate::extension_when::{Context, Predicate};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub const PROTOCOL: &str = "loom.extension.v1";
pub const API: &str = "1.0";
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_INTEGER: u64 = 9_007_199_254_740_991;
const LISTS: [&str; 11] = [
    "commands",
    "shortcuts",
    "menus",
    "settings",
    "dataTypes",
    "renderers",
    "unitOverlays",
    "backgroundTasks",
    "resourceProviders",
    "diagnostics",
    "eventSubscriptions",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Budget,
    Encoding,
    Protocol,
    Identity,
    Scope,
    Predicate,
    Permission,
}
#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Trust {
    Trusted,
    UnsignedDeveloper,
    Revoked,
    Untrusted,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Plugin {
    id: String,
    version: String,
    package_digest: String,
    trust_status: Trust,
    permission_grant_digest: String,
    #[serde(default, deserialize_with = "permissions")]
    effective_permissions: Vec<String>,
    scope_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Contribution {
    id: String,
    plugin_id: String,
    scope_id: String,
    #[serde(default, deserialize_with = "optional_text")]
    title: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    command_id: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    when: Option<String>,
    #[serde(default, deserialize_with = "optional_text")]
    placement: Option<String>,
    #[serde(default, deserialize_with = "optional_order")]
    order: Option<i32>,
    payload: Option<Value>,
    #[serde(skip)]
    predicate: Option<Predicate>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Wire {
    protocol: String,
    api_version: String,
    generation: u64,
    plugins: Vec<Plugin>,
    contributions: BTreeMap<String, Vec<Contribution>>,
}
pub struct Snapshot(Wire, [u8; 32]);
#[path = "loom_extension_declarations.rs"]
mod declarations;
pub use declarations::{Declaration, Owner};
#[path = "loom_extension_shortcuts.rs"]
mod shortcuts;
pub use shortcuts::{Chord, Shortcut};
#[path = "loom_extension_toolbar.rs"]
mod toolbar;
#[cfg(test)]
pub(crate) use toolbar::tests::fixture as toolbar_fixture;
pub use toolbar::ToolbarItem;
pub struct Command<'a> {
    owner: &'a Plugin,
    contribution: &'a Contribution,
}
pub(super) fn text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.encode_utf16().count() <= max
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn permissions<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Ok(Option::<Vec<String>>::deserialize(d)?.unwrap_or_default())
}
fn optional_text<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn optional_order<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i32>, D::Error> {
    i32::deserialize(d).map(Some)
}
impl Snapshot {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        // 先限制 wire 输入，再核对 JSON.stringify 对应的紧凑快照字节预算。
        if bytes.len() > 64 * 1024 * 1024 {
            return Err(Error::Budget);
        }
        let compact: Value = serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?;
        let compact = serde_json::to_vec(&compact).map_err(|_| Error::Encoding)?;
        if compact.len() > MAX_BYTES {
            return Err(Error::Budget);
        }
        let fingerprint = ring::digest::digest(&ring::digest::SHA256, &compact);
        let mut result = Self(
            serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?,
            fingerprint.as_ref().try_into().unwrap(),
        );
        let version: Vec<_> = result.0.api_version.split('.').collect();
        if result.0.protocol != PROTOCOL
            || version.len() != 2
            || version[0] != "1"
            || version[1].is_empty()
            || !version[1].bytes().all(|b| b.is_ascii_digit())
            || !text(&result.0.api_version, 32)
            || result.0.generation > MAX_INTEGER
        {
            return Err(Error::Protocol);
        }
        result.validate()?;
        Ok(result)
    }
    fn validate(&mut self) -> Result<(), Error> {
        if self.0.plugins.len() > 128
            || self.0.contributions.len() != LISTS.len()
            || LISTS
                .iter()
                .any(|key| !self.0.contributions.contains_key(*key))
        {
            return Err(Error::Budget);
        }
        let mut scopes = BTreeMap::new();
        for plugin in &self.0.plugins {
            if !text(&plugin.id, 384)
                || !text(&plugin.version, 128)
                || !text(&plugin.scope_id, 384)
                || !digest(&plugin.package_digest)
                || !digest(&plugin.permission_grant_digest)
                || scopes
                    .insert(plugin.id.as_str(), plugin.scope_id.as_str())
                    .is_some()
            {
                return Err(Error::Identity);
            }
            let mut permissions = BTreeSet::new();
            if plugin.effective_permissions.len() > 64
                || plugin
                    .effective_permissions
                    .iter()
                    .any(|p| !text(p, 128) || !permissions.insert(p))
            {
                return Err(Error::Permission);
            }
        }
        let mut ids = BTreeSet::new();
        let mut total = 0;
        for list in self.0.contributions.values_mut() {
            total += list.len();
            if list.len() > 256 || total > 2048 {
                return Err(Error::Budget);
            }
            for item in list {
                if !text(&item.id, 384)
                    || !text(&item.plugin_id, 384)
                    || !text(&item.scope_id, 384)
                    || !ids.insert(item.id.to_lowercase())
                    || item.order.is_some_and(|v| !(-10000..=10000).contains(&v))
                    || [
                        (item.title.as_deref(), 128),
                        (item.command_id.as_deref(), 384),
                        (item.when.as_deref(), 2048),
                        (item.placement.as_deref(), 128),
                    ]
                    .iter()
                    .any(|(value, max)| value.is_some_and(|s| !text(s, *max)))
                {
                    return Err(Error::Identity);
                }
                if scopes.get(item.plugin_id.as_str()).copied() != Some(item.scope_id.as_str()) {
                    return Err(Error::Scope);
                }
                item.predicate =
                    Some(Predicate::compile(item.when.as_deref()).map_err(|_| Error::Predicate)?);
            }
        }
        Ok(())
    }
    pub fn generation(&self) -> u64 {
        self.0.generation
    }
    /// 独立执行连接必须匹配目录的完整来源，不能只比较可能在 daemon 重启后复用的 generation。
    pub fn same_catalog(&self, other: &Self) -> bool {
        self.1 == other.1
    }
    pub fn command(&self, id: &str, context: &Context<'_>) -> Option<Command<'_>> {
        let contribution = self.0.contributions["commands"]
            .iter()
            .rev()
            .find(|item| item.command_id.as_deref().unwrap_or(&item.id) == id)?;
        let owner = self.0.plugins.iter().find(|owner| {
            owner.id == contribution.plugin_id
                && owner.scope_id == contribution.scope_id
                && owner.trust_status == Trust::Trusted
        })?;
        contribution
            .predicate
            .as_ref()
            .unwrap()
            .evaluate(context)
            .then_some(Command {
                owner,
                contribution,
            })
    }
}
impl Command<'_> {
    pub fn id(&self) -> &str {
        self.contribution
            .command_id
            .as_deref()
            .unwrap_or(&self.contribution.id)
    }
    pub fn plugin(&self) -> &str {
        &self.owner.id
    }
    pub fn scope(&self) -> &str {
        &self.owner.scope_id
    }
    pub fn payload(&self) -> Option<&serde_json::Map<String, Value>> {
        self.contribution.payload()
    }
    /// 声明的权限必须由同 scope 的有效 grant 覆盖；此处不签发资源许可。
    pub fn permits(&self, permission: &str) -> bool {
        self.permissions()
            .is_some_and(|items| items.contains(&permission))
    }
    pub fn permissions(&self) -> Option<Vec<&str>> {
        let items = self.payload()?.get("permissions")?.as_array()?;
        if items.len() > 64 {
            return None;
        }
        items
            .iter()
            .map(|item| {
                item.as_str().filter(|s| {
                    text(s, 128)
                        && self
                            .owner
                            .effective_permissions
                            .iter()
                            .any(|granted| granted.as_str() == *s)
                })
            })
            .collect()
    }
}
impl Contribution {
    fn payload(&self) -> Option<&serde_json::Map<String, Value>> {
        let outer = self.payload.as_ref()?.as_object()?;
        if outer.contains_key("schema") && outer.contains_key("payload") {
            if let Some(nested) = outer["payload"].as_object() {
                return Some(nested);
            }
        }
        Some(outer)
    }
}
#[cfg(test)]
#[path = "loom_extension_snapshot_tests.rs"]
pub(crate) mod tests;
