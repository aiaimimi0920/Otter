//! 连线显示值的有界复制；先计费再分配，不先克隆远端大字符串/容器再拒绝。
use crate::graph_image::Output;
use serde_json::{Map, Value};
use std::io::{self, Write};

struct Budget {
    bytes: usize,
    nodes: usize,
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_sub(bytes.len())
            .ok_or_else(|| io::Error::other("display value budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Budget {
    fn charge(&mut self, value: impl serde::Serialize) -> Result<(), ()> {
        serde_json::to_writer(self, &value).map_err(|_| ())
    }
    fn copy(&mut self, value: Output<'_>, depth: usize) -> Result<Option<Value>, ()> {
        if depth > 32 {
            return Err(());
        }
        self.nodes = self.nodes.checked_sub(1).ok_or(())?;
        Ok(Some(match value {
            Output::Image(text) => {
                self.charge(text)?;
                Value::String(text.into())
            }
            Output::Number(number) => {
                let value = serde_json::Number::from_f64(number).ok_or(())?;
                self.charge(&value)?;
                Value::Number(value)
            }
            Output::Boolean(value) => {
                self.charge(value)?;
                Value::Bool(value)
            }
            Output::Null => {
                self.charge(Value::Null)?;
                Value::Null
            }
            Output::Other => return Ok(None),
            Output::Json(value) => match value {
                Value::Array(values) => self.array(values.iter().map(Output::Json), depth)?,
                Value::Object(values) => self.object(
                    values
                        .iter()
                        .map(|(key, value)| (key.as_str(), Output::Json(value))),
                    depth,
                )?,
                _ => {
                    self.charge(value)?;
                    value.clone()
                }
            },
            Output::Array(values) => self.array(values.iter().copied(), depth)?,
            Output::Object(values) => self.object(values.iter().copied(), depth)?,
        }))
    }
    fn array<'a>(
        &mut self,
        values: impl ExactSizeIterator<Item = Output<'a>>,
        depth: usize,
    ) -> Result<Value, ()> {
        if values.len() > self.nodes {
            return Err(());
        }
        self.write_all(b"[]").map_err(|_| ())?;
        let mut result = Vec::new();
        result.try_reserve(values.len()).map_err(|_| ())?;
        for (index, value) in values.enumerate() {
            if index > 0 {
                self.write_all(b",").map_err(|_| ())?;
            }
            match self.copy(value, depth + 1)? {
                Some(value) => result.push(value),
                None => {
                    self.charge(Value::Null)?;
                    result.push(Value::Null);
                }
            }
        }
        Ok(Value::Array(result))
    }
    fn object<'a>(
        &mut self,
        values: impl ExactSizeIterator<Item = (&'a str, Output<'a>)>,
        depth: usize,
    ) -> Result<Value, ()> {
        if values.len() > 128 || values.len() > self.nodes {
            return Err(());
        }
        self.write_all(b"{}").map_err(|_| ())?;
        let mut result = Map::new();
        for (key, value) in values {
            if key.len() > 256 || result.contains_key(key) {
                return Err(());
            }
            if matches!(value, Output::Other) {
                continue;
            }
            if !result.is_empty() {
                self.write_all(b",").map_err(|_| ())?;
            }
            self.charge(key)?;
            self.write_all(b":").map_err(|_| ())?;
            if let Some(value) = self.copy(value, depth + 1)? {
                result.insert(key.into(), value);
            }
        }
        Ok(Value::Object(result))
    }
}
pub(super) fn capture<'a>(
    values: impl Iterator<Item = (&'a str, Option<Output<'a>>)>,
) -> Result<Map<String, Value>, &'static str> {
    let mut budget = Budget {
        bytes: crate::loom_parameter_state::MAX_BYTES,
        nodes: 4096,
    };
    let mut result = Map::new();
    for (index, (name, value)) in values.enumerate() {
        if index >= 128 || name.is_empty() || name.len() > 256 || name.chars().any(char::is_control)
        {
            return Err("有效参数显示字段无效");
        }
        let Some(value) = value else {
            continue;
        };
        budget.charge(name).map_err(|_| "有效参数显示超过预算")?;
        if let Some(value) = budget.copy(value, 0).map_err(|_| "有效参数显示超过预算")? {
            result.insert(name.into(), value);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_nested_json_and_undefined_conventions() {
        let object = [
            ("missing", Output::Other),
            (
                "array",
                Output::Array(&[Output::Other, Output::Boolean(false)]),
            ),
        ];
        let result = capture([("value", Some(Output::Object(&object)))].into_iter()).unwrap();
        assert_eq!(result["value"], serde_json::json!({"array":[null,false]}));
        let json = serde_json::json!({"text":"中文\n", "n":1, "large":u64::MAX});
        assert_eq!(
            capture([("v", Some(Output::Json(&json)))].into_iter()).unwrap()["v"],
            json
        );
    }
    #[test]
    fn rejects_escaped_byte_budget_and_aggregate_nodes() {
        let oversized = "\u{0001}".repeat(crate::loom_parameter_state::MAX_BYTES / 6);
        assert!(capture([("v", Some(Output::Image(&oversized)))].into_iter()).is_err());
        let values = vec![Output::Null; 3000];
        assert!(capture(
            [
                ("a", Some(Output::Array(&values))),
                ("b", Some(Output::Array(&values)))
            ]
            .into_iter()
        )
        .is_err());
    }
    #[test]
    fn oversized_borrowed_string_is_rejected_before_copy() {
        let value = "x".repeat(crate::loom_parameter_state::MAX_BYTES);
        let mut budget = Budget {
            bytes: 16,
            nodes: 4096,
        };
        assert!(budget.copy(Output::Image(&value), 0).is_err());
        assert!(capture([("", Some(Output::Null))].into_iter()).is_err());
    }
    #[test]
    fn depth_limit_is_checked_before_descending_into_containers() {
        let mut value = Value::Null;
        for _ in 0..32 {
            value = Value::Array(vec![value]);
        }
        assert!(capture([("v", Some(Output::Json(&value)))].into_iter()).is_ok());
        value = Value::Array(vec![value]);
        assert!(capture([("v", Some(Output::Json(&value)))].into_iter()).is_err());
    }
}
