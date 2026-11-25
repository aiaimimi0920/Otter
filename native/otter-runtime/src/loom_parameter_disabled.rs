//! Stash original disabled public-parameter values; the stash and effective parameters each obey the same budget, and stashed values do not enter execution wire.
use super::*;
impl Parameters {
    pub fn toggle_disabled(&mut self, id: &str, fallback: Value) -> Result<bool, Error> {
        let previous = self.get(id).ok_or(Error::UnknownParameter)?;
        if is_disabled(previous) {
            let restored = self.disabled_values.get(id).cloned().unwrap_or(fallback);
            return self.set(id, Some(restored));
        }
        // The stash has independent limits of 1 MiB/4096 nodes/128 entries; successive large parameters must not bypass the total budget.
        validate(
            self.disabled_values
                .iter()
                .filter(|(name, _)| name.as_str() != id)
                .map(|(name, value)| (name.as_str(), Some(value)))
                .chain(previous.map(|value| (id, Some(value)))),
        )?;
        let backup = previous.cloned();
        let changed = self.set(
            id,
            Some(Value::String(crate::graph_params::DISABLED.into())),
        )?;
        if let Some(backup) = backup {
            self.disabled_values.insert(id.into(), backup);
        }
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
    fn leaving_disabled_releases_stash_for_the_next_parameter() {
        for exit in 0..3 {
            let mut values = parameters();
            let big = Value::String("a".repeat(600_000));
            values.set("value", Some(big.clone())).unwrap();
            values.toggle_disabled("value", Value::Null).unwrap();
            match exit {
                0 => {
                    values.toggle_disabled("value", Value::Null).unwrap();
                    assert_eq!(values.get("value"), Some(Some(&big)));
                    values.set("value", None).unwrap();
                }
                1 => {
                    values.set("value", None).unwrap();
                }
                _ => {
                    values.replace(serde_json::Map::new()).unwrap();
                }
            }
            assert!(
                !values.disabled_values.contains_key("value"),
                "exit path {exit}"
            );
            values.set("other", Some(big)).unwrap();
            assert!(values.toggle_disabled("other", Value::Null).unwrap());
        }
    }

    #[test]
    fn rejected_enable_and_batch_edit_leave_the_disabled_backup_intact() {
        let mut values = parameters();
        let big = Value::String("a".repeat(600_000));
        values.set("value", Some(big.clone())).unwrap();
        values.toggle_disabled("value", Value::Null).unwrap();
        values.set("other", Some(big.clone())).unwrap();
        let revision = values.revision();
        assert_eq!(
            values.toggle_disabled("value", Value::Null),
            Err(Error::Capacity)
        );
        assert_eq!(
            values.replace(serde_json::Map::from_iter([
                ("value".into(), big.clone()),
                ("other".into(), big.clone()),
            ])),
            Err(Error::Capacity)
        );
        assert_eq!(values.revision(), revision);
        assert_eq!(values.disabled_values.get("value"), Some(&big));
        values.set("other", None).unwrap();
        assert!(values.toggle_disabled("value", Value::Null).unwrap());
        assert_eq!(values.get("value"), Some(Some(&big)));
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
