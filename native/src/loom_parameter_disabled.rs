//! 公共参数的禁用原值暂存；与有效参数各自受同样预算约束，不进入执行 wire。
use super::*;
impl Parameters {
    pub fn toggle_disabled(&mut self, id: &str, fallback: Value) -> Result<bool, Error> {
        let previous = self.get(id).ok_or(Error::UnknownParameter)?;
        if previous.and_then(Value::as_str) == Some(crate::graph_params::DISABLED) {
            let restored = self.disabled_values.get(id).cloned().unwrap_or(fallback);
            return self.set(id, Some(restored));
        }
        // 暂存区独立上限 1 MiB/4096 节点/128 项；不能逐次塞入多个大参数绕过总预算。
        let mut stash = self.disabled_values.clone();
        if let Some(value) = previous {
            stash.insert(id.into(), value.clone());
        } else {
            stash.remove(id);
        }
        validate(
            stash
                .iter()
                .map(|(name, value)| (name.as_str(), Some(value))),
        )?;
        let changed = self.set(
            id,
            Some(Value::String(crate::graph_params::DISABLED.into())),
        )?;
        self.disabled_values = stash;
        Ok(changed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> Parameters {
        let raw = serde_json::json!({"id":"art","parameters":[{"id":"value","default":null},{"id":"other"},{"id":"token","secret":true}]});
        Parameters::new(&Capability::new(&raw).unwrap()).unwrap()
    }
    #[test]
    fn toggle_restores_null_and_uses_fallback_only_for_unset_values() {
        let mut values = parameters();
        values
            .toggle_disabled("value", Value::String("fallback".into()))
            .unwrap();
        assert_eq!(
            values.get("value").flatten().and_then(Value::as_str),
            Some(crate::graph_params::DISABLED)
        );
        values
            .toggle_disabled("value", Value::String("fallback".into()))
            .unwrap();
        assert_eq!(values.get("value"), Some(Some(&Value::Null)));
        values.toggle_disabled("other", Value::Bool(false)).unwrap();
        values.toggle_disabled("other", Value::Bool(false)).unwrap();
        assert_eq!(values.get("other"), Some(Some(&Value::Bool(false))));
        assert_eq!(
            values.toggle_disabled("token", Value::Null),
            Err(Error::UnknownParameter)
        );
    }
    #[test]
    fn combined_stash_budget_failure_is_atomic() {
        let mut values = parameters();
        let big = Value::String("a".repeat(600_000));
        values.set("value", Some(big.clone())).unwrap();
        values.toggle_disabled("value", Value::Null).unwrap();
        values.set("other", Some(big)).unwrap();
        let revision = values.revision();
        assert_eq!(
            values.toggle_disabled("other", Value::Null),
            Err(Error::Capacity)
        );
        assert_eq!(values.revision(), revision);
        assert_eq!(
            values
                .get("other")
                .flatten()
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            600_000
        );
        values.set("other", None).unwrap();
        values.toggle_disabled("value", Value::Null).unwrap();
        assert_eq!(
            values
                .get("value")
                .flatten()
                .unwrap()
                .as_str()
                .unwrap()
                .len(),
            600_000
        );
    }
}
