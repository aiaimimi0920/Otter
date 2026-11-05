//! Loom 拥有触发执行；观看端严格恢复注册快照，不在本地重新求值或执行。
use crate::{
    live_relay_observation::Confidence,
    live_relay_viewer_identity::{identifier, parse},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Invalid,
    Limit,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Equals,
    NotEquals,
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Contains,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Condition {
    pub condition_id: String,
    pub revision: u64,
    pub observation_id: String,
    pub operator: Operator,
    pub operand: Value,
    pub stable_for_ms: u32,
    pub rising_edge: bool,
    pub rearm: bool,
    pub minimum_confidence: Confidence,
}
impl Condition {
    pub fn validate(&self) -> Result<(), Error> {
        if !identifier(&self.condition_id)
            || !identifier(&self.observation_id)
            || self.revision == 0
            || self.stable_for_ms > 86_400_000
        {
            return Err(Error::Invalid);
        }
        if serde_json::to_vec(&self.operand)
            .map_err(|_| Error::Json)?
            .len()
            > 4096
            || !bounded(&self.operand, 1, &mut 0)
        {
            return Err(Error::Limit);
        }
        let mut expected = &self.operand;
        if let Some(object) = self.operand.as_object() {
            if object.contains_key("path") || object.contains_key("value") {
                if object.len() != 2
                    || !object
                        .get("path")
                        .and_then(Value::as_str)
                        .is_some_and(|path| {
                            path.starts_with('/') && path != "/" && path.len() <= 256
                        })
                {
                    return Err(Error::Invalid);
                }
                expected = object.get("value").ok_or(Error::Invalid)?;
            }
        }
        if matches!(
            self.operator,
            Operator::GreaterThan
                | Operator::GreaterOrEqual
                | Operator::LessThan
                | Operator::LessOrEqual
        ) && expected.as_f64().is_none()
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
}
// 深度最多 16，递归栈固定有界；不为宽数组建立同样宽的临时队列。
fn bounded(value: &Value, depth: usize, nodes: &mut usize) -> bool {
    *nodes += 1;
    if depth > 16 || *nodes > 512 {
        return false;
    }
    match value {
        Value::Array(values) => values.iter().all(|v| bounded(v, depth + 1, nodes)),
        Value::Object(values) => values.values().all(|v| bounded(v, depth + 1, nodes)),
        _ => true,
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Target {
    pub surface_instance_id: String,
    pub surface_attachment_id: String,
    pub surface_node_id: String,
    pub surface_event: String,
    pub surface_action: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub binding_id: String,
    pub observation_id: String,
    pub condition_revision: u64,
    pub authorized_by: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Registration {
    pub binding: Binding,
    pub condition: Condition,
    pub target: Target,
    pub armed: bool,
    pub last_match: bool,
    pub pending_offline_match: bool,
}
impl Registration {
    pub fn validate(&self) -> Result<(), Error> {
        let b = &self.binding;
        let t = &self.target;
        if [
            &b.binding_id,
            &b.observation_id,
            &b.authorized_by,
            &t.surface_instance_id,
            &t.surface_attachment_id,
            &t.surface_node_id,
            &t.surface_event,
            &t.surface_action,
        ]
        .iter()
        .any(|v| !identifier(v))
            || b.condition_revision == 0
            || b.condition_revision != self.condition.revision
            || b.observation_id != self.condition.observation_id
        {
            return Err(Error::Invalid);
        }
        self.condition.validate()
    }
}
#[derive(Deserialize)]
struct Snapshot {
    #[serde(default)]
    triggers: Vec<Registration>,
}
pub fn from_snapshot(bytes: &[u8]) -> Result<Vec<Registration>, Error> {
    let snapshot: Snapshot = parse(bytes).map_err(|_| Error::Json)?;
    if snapshot.triggers.len() > 64 {
        return Err(Error::Limit);
    }
    let mut ids = std::collections::HashSet::new();
    for registration in &snapshot.triggers {
        registration.validate()?;
        if !ids.insert(&registration.binding.binding_id) {
            return Err(Error::Invalid);
        }
    }
    Ok(snapshot.triggers)
}
