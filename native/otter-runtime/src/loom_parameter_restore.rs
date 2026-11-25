//! 会话参数初始化：保留当前能力未声明的公共值，但不把它们变成可编辑声明或连线端口。
use super::*;
impl Parameters {
    pub fn from_persisted(
        capability: &Capability<'_>,
        mut values: serde_json::Map<String, Value>,
    ) -> Result<Self, Error> {
        let mut candidate = Self::new(capability)?;
        let secret: Vec<_> = capability
            .parameters()
            .filter(|parameter| parameter.secret)
            .map(|parameter| parameter.id)
            .collect();
        values.retain(|id, _| !crate::graph_params::internal(id) && !secret.contains(&id.as_str()));
        // 先验证借用候选，再移入 owner；未知字段与声明字段共享同一字节/节点/条目预算。
        validate(
            candidate
                .values
                .keys()
                .map(|id| (id.as_str(), values.get(id)))
                .chain(
                    values
                        .iter()
                        .filter(|(id, _)| !candidate.values.contains_key(*id))
                        .map(|(id, value)| (id.as_str(), Some(value))),
                ),
        )?;
        for (id, value) in &mut candidate.values {
            *value = values.remove(id);
        }
        candidate.retained = values.into_iter().collect();
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_values_survive_declared_edits_without_becoming_controls() {
        let definition = serde_json::json!({"id":"a","parameters":[{"id":"value","default":1},{"id":"secret","secret":true}]});
        let capability = Capability::new(&definition).unwrap();
        let values =
            serde_json::json!({"legacy":{"n":42},"secret":"hidden","__exec_mode":"ignore"})
                .as_object()
                .unwrap()
                .clone();
        let mut parameters = Parameters::from_persisted(&capability, values).unwrap();
        assert_eq!(parameters.get("value"), Some(None));
        assert!(parameters.get("legacy").is_none());
        assert!(parameters
            .entries()
            .any(|(id, value)| id == "legacy" && value == Some(&serde_json::json!({"n":42}))));
        assert!(!parameters
            .entries()
            .any(|(id, _)| id == "secret" || id == "__exec_mode"));
        parameters.set("value", Some(Value::from(2))).unwrap();
        assert_eq!(
            parameters.set("legacy", Some(Value::Null)),
            Err(Error::UnknownParameter)
        );
        let replacement = serde_json::json!({"legacy":false,"value":3})
            .as_object()
            .unwrap()
            .clone();
        parameters.replace(replacement).unwrap();
        assert!(parameters
            .entries()
            .any(|(id, value)| id == "legacy" && value == Some(&Value::Bool(false))));
        parameters.replace(serde_json::Map::new()).unwrap();
        assert!(!parameters.entries().any(|(id, _)| id == "legacy"));
    }
    #[test]
    fn retained_values_cannot_bypass_shared_budget() {
        let definition = serde_json::json!({"id":"a","parameters":[{"id":"value"}]});
        let capability = Capability::new(&definition).unwrap();
        let values = serde_json::json!({"legacy":"x".repeat(MAX_BYTES)})
            .as_object()
            .unwrap()
            .clone();
        assert!(matches!(
            Parameters::from_persisted(&capability, values),
            Err(Error::Capacity)
        ));
    }
}
