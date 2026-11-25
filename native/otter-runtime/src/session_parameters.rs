//! 会话 Art 参数恢复；能力定义决定 secret 边界，未知公共值保留并受统一预算约束。
use crate::{loom_capability::Capability, loom_parameter_state::Parameters};
use serde_json::{value::RawValue, Map, Value};
#[derive(Debug, PartialEq)]
pub enum Error {
    Capacity,
    Shape,
    Definition,
    State,
}

/// raw 从会话原文借用；先检查字节，再解析公共候选。缺失参数保持未设置，由执行层回退声明默认值。
pub fn restore(capability: &Capability<'_>, raw: Option<&RawValue>) -> Result<Parameters, Error> {
    let values: Map<String, Value> = if let Some(raw) = raw {
        if raw.get().len() > crate::loom_parameter_state::MAX_BYTES {
            return Err(Error::Capacity);
        }
        serde_json::from_str(raw.get()).map_err(|_| Error::Shape)?
    } else {
        Map::new()
    };
    Parameters::from_persisted(capability, values).map_err(|error| match error {
        crate::loom_parameter_state::Error::Capacity => Error::Capacity,
        crate::loom_parameter_state::Error::Definition => Error::Definition,
        _ => Error::State,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_manual_values_without_secret_or_control_fields() {
        let definition = serde_json::json!({"id":"art","parameters":[
            {"id":"amount","type":"number","default":5},{"id":"empty","type":"string"},
            {"id":"off","type":"bool"},{"id":"secret","secret":true}
        ]});
        let capability = Capability::new(&definition).unwrap();
        let raw = RawValue::from_string(r#"{"empty":null,"off":"__DISABLED__","secret":"do-not-restore","__exec_mode":"ignore"}"#.into()).unwrap();
        let restored = restore(&capability, Some(&raw)).unwrap();
        assert_eq!(restored.get("amount"), Some(None));
        assert_eq!(restored.get("empty"), Some(Some(&Value::Null)));
        assert_eq!(
            restored.get("off"),
            Some(Some(&Value::String(crate::graph_params::DISABLED.into())))
        );
        assert!(restored.get("secret").is_none());
    }
    #[test]
    fn unknown_public_values_and_nonobject_shapes_are_not_silently_discarded() {
        let definition = serde_json::json!({"id":"art","parameters":[{"id":"value","default":1}]});
        let capability = Capability::new(&definition).unwrap();
        let unknown = RawValue::from_string(r#"{"removedParameter":42}"#.into()).unwrap();
        assert!(restore(&capability, Some(&unknown))
            .unwrap()
            .entries()
            .any(|(id, value)| id == "removedParameter" && value == Some(&Value::from(42))));
        let array = RawValue::from_string("[]".into()).unwrap();
        assert!(matches!(
            restore(&capability, Some(&array)),
            Err(Error::Shape)
        ));
        assert_eq!(restore(&capability, None).unwrap().get("value"), Some(None));
    }
}
