//! 文档公共参数 owner：有界初始化/修改先验证再提交，不保留 secret 或共享可变目录。
use crate::loom_capability::Capability;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};
pub(crate) const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
    Definition,
    UnknownParameter,
    RevisionExhausted,
}
#[derive(Clone)]
pub struct Parameters {
    retained: BTreeMap<String, Value>,
    disabled_values: BTreeMap<String, Value>,
    art_id: String,
    revision: u64,
    values: BTreeMap<String, Option<Value>>,
}
impl Parameters {
    pub fn new(capability: &Capability<'_>) -> Result<Self, Error> {
        if capability.id.len() > 256 || capability.id.chars().any(char::is_control) {
            return Err(Error::Definition);
        }
        let initial = capability
            .initial_parameters()
            .map_err(|_| Error::Definition)?;
        let entries = initial.iter().map(|(id, value)| (*id, value.as_deref()));
        validate(entries)?;
        Ok(Self {
            retained: BTreeMap::new(),
            disabled_values: BTreeMap::new(),
            art_id: capability.id.into(),
            revision: 0,
            values: initial
                .into_iter()
                .map(|(id, value)| (id.into(), value.map(|v| v.into_owned())))
                .collect(),
        })
    }
    pub fn art_id(&self) -> &str {
        &self.art_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// 外层 None 是未声明/非公共参数，内层 None 是已声明但未设置；null 是 Value::Null。
    pub fn get(&self, id: &str) -> Option<Option<&Value>> {
        self.values.get(id).map(Option::as_ref)
    }
    pub fn entries(&self) -> impl Iterator<Item = (&str, Option<&Value>)> {
        self.values
            .iter()
            .map(|(id, value)| (id.as_str(), value.as_ref()))
            .chain(
                self.retained
                    .iter()
                    .map(|(id, value)| (id.as_str(), Some(value))),
            )
    }
    /// 整批编辑：省略的公共项变为未设置；所有检查通过前不修改现有状态。
    pub fn replace(&mut self, mut values: serde_json::Map<String, Value>) -> Result<bool, Error> {
        if values
            .keys()
            .any(|id| !self.values.contains_key(id) && !self.retained.contains_key(id))
        {
            return Err(Error::UnknownParameter);
        }
        validate(
            self.values
                .keys()
                .chain(self.retained.keys())
                .map(|id| (id.as_str(), values.get(id))),
        )?;
        if self.entries().all(|(id, old)| old == values.get(id)) {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        for (id, value) in &mut self.values {
            *value = values.remove(id);
        }
        self.retained.retain(|id, value| {
            if let Some(next) = values.remove(id) {
                *value = next;
                true
            } else {
                false
            }
        });
        self.revision = revision;
        Ok(true)
    }
    pub fn set(&mut self, id: &str, value: Option<Value>) -> Result<bool, Error> {
        let previous = self.values.get(id).ok_or(Error::UnknownParameter)?;
        // 校验必须在深层 Value 比较前，防止调用方构造超深数据绕过预算。
        validate(
            self.entries()
                .map(|(name, old)| (name, if name == id { value.as_ref() } else { old })),
        )?;
        if *previous == value {
            return Ok(false);
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        *self.values.get_mut(id).expect("validated parameter") = value;
        self.revision = revision;
        Ok(true)
    }
}
struct Meter(usize);
impl Write for Meter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_BYTES)
            .ok_or_else(|| io::Error::other("parameter budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn structure(value: &Value, depth: usize, remaining: &mut usize) -> Result<(), Error> {
    if depth > 32 || *remaining == 0 {
        return Err(Error::Capacity);
    }
    *remaining -= 1;
    match value {
        Value::Array(values) => {
            for value in values {
                structure(value, depth + 1, remaining)?;
            }
        }
        Value::Object(fields) => {
            for value in fields.values() {
                structure(value, depth + 1, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}
pub(crate) fn validate<'a>(
    entries: impl Iterator<Item = (&'a str, Option<&'a Value>)>,
) -> Result<(), Error> {
    let mut meter = Meter(0);
    let mut remaining = 4096;
    for (index, (id, value)) in entries.enumerate() {
        if index >= 128 || id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) {
            return Err(Error::Capacity);
        }
        serde_json::to_writer(&mut meter, id).map_err(|_| Error::Capacity)?;
        if let Some(value) = value {
            structure(value, 0, &mut remaining)?;
            serde_json::to_writer(&mut meter, value).map_err(|_| Error::Capacity)?;
        }
    }
    Ok(())
}
#[path = "loom_parameter_disabled.rs"]
mod disabled;
#[path = "loom_parameter_restore.rs"]
mod restore;
#[cfg(test)]
#[path = "loom_parameter_state_tests.rs"]
mod tests;
