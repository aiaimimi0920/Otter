//! Restore original fields according to representation differences; f32/defaults/control ranges must not rewrite unedited wire values.
use super::*;
use std::collections::BTreeSet;
pub(super) fn merge(
    original: Option<&Value>,
    expected: Option<&Value>,
    current: Option<&Value>,
) -> Option<Value> {
    if expected == current {
        return original.cloned();
    }
    let current = current?;
    if let (Some(Value::Object(original)), Some(Value::Object(expected)), Value::Object(current)) =
        (original, expected, current)
    {
        let keys: BTreeSet<_> = original
            .keys()
            .chain(expected.keys())
            .chain(current.keys())
            .collect();
        return Some(Value::Object(
            keys.into_iter()
                .filter_map(|key| {
                    merge(original.get(key), expected.get(key), current.get(key))
                        .map(|v| (key.clone(), v))
                })
                .collect(),
        ));
    }
    if let (Some(Value::Array(original)), Some(Value::Array(expected)), Value::Array(current)) =
        (original, expected, current)
    {
        return Some(Value::Array(
            current
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    merge(original.get(i), expected.get(i), Some(v)).unwrap_or_else(|| v.clone())
                })
                .collect(),
        ));
    }
    Some(current.clone())
}
pub(super) fn wire(value: &mut Value) -> Result<(), Error> {
    let is_arrow = value["type"] == "arrow";
    if let Some(show) = value
        .as_object_mut()
        .ok_or(Error::Unsupported)?
        .remove("showArrowHead")
    {
        if show != false && !is_arrow {
            return Err(Error::Unsupported);
        }
    }
    Ok(())
}
