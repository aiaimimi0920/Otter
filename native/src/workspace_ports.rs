//! 端口显示是独立外观状态：不修改拓扑/执行凭证；旧同名输入输出共用用户覆盖。
use super::{Error, UnitId, Workspace};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::BTreeMap, fmt, sync::Arc};
pub const MAX_PORTS: usize = 512;
pub const MAX_BYTES: usize = 32 * 1024;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Visibility(Arc<BTreeMap<String, bool>>);
impl Serialize for Visibility {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl Visibility {
    pub fn read(raw: &str) -> Result<Self, Error> {
        if raw.len() > MAX_BYTES {
            return Err(Error::Capacity);
        }
        serde_json::from_str(raw).map_err(|_| Error::InvalidPort)
    }
    pub fn get(&self, name: &str) -> Option<bool> {
        self.0.get(name).copied()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn edited(&self, name: &str, value: Option<bool>) -> Result<Self, Error> {
        if !super::valid_port(name) {
            return Err(Error::InvalidPort);
        }
        let mut next = (*self.0).clone();
        if let Some(value) = value {
            next.insert(name.into(), value);
        } else {
            next.remove(name);
        }
        if next.len() > MAX_PORTS
            || serde_json::to_vec(&next)
                .map_err(|_| Error::Allocation)?
                .len()
                > MAX_BYTES
        {
            return Err(Error::Capacity);
        }
        Ok(Self(Arc::new(next)))
    }
}
impl<'de> Deserialize<'de> for Visibility {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Map;
        impl<'de> de::Visitor<'de> for Map {
            type Value = Visibility;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded port visibility object")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.len() == MAX_PORTS
                        || !super::valid_port(&key)
                        || values.contains_key(&key)
                    {
                        return Err(de::Error::custom(
                            "invalid or duplicate port visibility key",
                        ));
                    }
                    values.insert(key, map.next_value::<bool>()?);
                }
                if serde_json::to_vec(&values)
                    .map_err(de::Error::custom)?
                    .len()
                    > MAX_BYTES
                {
                    return Err(de::Error::custom("port visibility byte limit exceeded"));
                }
                Ok(Visibility(Arc::new(values)))
            }
        }
        decoder.deserialize_map(Map)
    }
}
#[derive(Clone)]
pub struct AppearanceStamp(Arc<()>);
impl AppearanceStamp {
    pub fn matches(&self, workspace: &Workspace) -> bool {
        Arc::ptr_eq(&self.0, &workspace.appearance)
    }
}
impl Workspace {
    pub fn appearance_stamp(&self) -> AppearanceStamp {
        AppearanceStamp(self.appearance.clone())
    }
    pub fn port_visibility(&self, unit: &UnitId) -> Visibility {
        self.port_visibility
            .iter()
            .find(|(id, _)| id == unit)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }
    pub fn visibility_records(&self) -> &[(UnitId, Visibility)] {
        &self.port_visibility
    }
    pub fn set_port_visibility(&mut self, unit: &UnitId, value: Visibility) -> Result<bool, Error> {
        if !self.contains(unit) {
            return Err(Error::MissingUnit);
        }
        if self.port_visibility(unit) == value {
            return Ok(false);
        }
        if let Some(index) = self.port_visibility.iter().position(|(id, _)| id == unit) {
            if value.is_empty() {
                self.port_visibility.remove(index);
            } else {
                self.port_visibility[index].1 = value;
            }
        } else if !value.is_empty() {
            self.port_visibility
                .try_reserve(1)
                .map_err(|_| Error::Allocation)?;
            self.port_visibility.push((unit.clone(), value));
        }
        self.appearance = Arc::new(());
        Ok(true)
    }
}
#[cfg(test)]
#[path = "workspace_ports_tests.rs"]
mod tests;
