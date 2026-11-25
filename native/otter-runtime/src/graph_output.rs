//! Borrow protocol values only, without script execution or image-path access; wrapper-field order matches the baseline getObjectField.
use super::Error;
pub const MAX_VALUE_NODES: usize = 4096;
pub const MAX_VALUE_DEPTH: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Output<'a> {
    /// String output (legacy name retained); consumers decide whether it is a usable image reference.
    Image(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
    Object(&'a [(&'a str, Output<'a>)]),
    Array(&'a [Output<'a>]),
    /// Borrowed view of a decoded JSON container; do not copy the entire output tree for graph resolution.
    Json(&'a serde_json::Value),
    /// Legacy sentinel for no serializable value; omit JSON object fields and encode array positions as null.
    Other,
}
impl<'a> Output<'a> {
    pub fn from_json(value: &'a serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Boolean(*value),
            serde_json::Value::Number(value) => {
                value.as_f64().map(Self::Number).unwrap_or(Self::Other)
            }
            serde_json::Value::String(value) => Self::Image(value),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => Self::Json(value),
        }
    }
    /// Unwrap only one level; false, 0, and empty strings are present values and must not be skipped to fetch a later src.
    pub fn direct(self) -> Self {
        if let Self::Json(serde_json::Value::Object(fields)) = self {
            for field in ["value", "data", "src", "previewSrc", "url", "path"] {
                if let Some(value) = fields.get(field).filter(|value| !value.is_null()) {
                    return Self::from_json(value);
                }
            }
        }
        if let Self::Object(fields) = self {
            for field in ["value", "data", "src", "previewSrc", "url", "path"] {
                if let Some((_, value)) = fields.iter().find(|(name, _)| *name == field) {
                    if !matches!(value, Self::Null) {
                        return *value;
                    }
                }
            }
        }
        self
    }
}
/// JSON decoding adapters must still limit input bytes; this limits structural traversal work in resolved graph views.
pub fn validate(values: &[(&str, Output<'_>)]) -> Result<(), Error> {
    let mut remaining = MAX_VALUE_NODES;
    for (index, (name, value)) in values.iter().enumerate() {
        if values[..index].iter().any(|(previous, _)| previous == name) {
            return Err(Error::InvalidOutput);
        }
        visit(*value, 0, &mut remaining)?;
    }
    Ok(())
}
fn visit(value: Output<'_>, depth: usize, remaining: &mut usize) -> Result<(), Error> {
    if depth > MAX_VALUE_DEPTH || *remaining == 0 {
        return Err(Error::Capacity);
    }
    *remaining -= 1;
    match value {
        Output::Json(value) => match value {
            serde_json::Value::Object(fields) => {
                if fields.len() > super::MAX_PORTS || fields.len() > *remaining {
                    return Err(Error::Capacity);
                }
                for (name, value) in fields {
                    if name.len() > 256 {
                        return Err(Error::InvalidOutput);
                    }
                    visit(Output::from_json(value), depth + 1, remaining)?;
                }
            }
            serde_json::Value::Array(values) => {
                if values.len() > *remaining {
                    return Err(Error::Capacity);
                }
                for value in values {
                    visit(Output::from_json(value), depth + 1, remaining)?;
                }
            }
            _ => visit(Output::from_json(value), depth, remaining)?,
        },
        Output::Object(fields) => {
            if fields.len() > super::MAX_PORTS || fields.len() > *remaining {
                return Err(Error::Capacity);
            }
            for (index, (name, value)) in fields.iter().enumerate() {
                if name.len() > 256 || fields[..index].iter().any(|(previous, _)| previous == name)
                {
                    return Err(Error::InvalidOutput);
                }
                visit(*value, depth + 1, remaining)?;
            }
        }
        Output::Array(values) => {
            if values.len() > *remaining {
                return Err(Error::Capacity);
            }
            for value in values {
                visit(*value, depth + 1, remaining)?;
            }
        }
        _ => {}
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapper_order_skips_null_but_not_false_zero_or_empty() {
        for value in [
            Output::Boolean(false),
            Output::Number(0.0),
            Output::Image(""),
        ] {
            let fields = [("src", Output::Image("wrong")), ("value", value)];
            assert_eq!(Output::Object(&fields).direct(), value);
        }
        assert_eq!(
            Output::Object(&[
                ("path", Output::Image("path")),
                ("value", Output::Null),
                ("data", Output::Image("data"))
            ])
            .direct(),
            Output::Image("data")
        );
    }
    #[test]
    fn wrapper_is_not_recursive_and_special_keys_are_only_data() {
        let inner = [("src", Output::Image("nested"))];
        let outer = [
            ("value", Output::Object(&inner)),
            ("src", Output::Image("wrong")),
        ];
        assert_eq!(Output::Object(&outer).direct(), Output::Object(&inner));
        let unrelated = [("__proto__", Output::Image("not-inherited"))];
        assert_eq!(
            Output::Object(&unrelated).direct(),
            Output::Object(&unrelated)
        );
    }
    #[test]
    fn structural_limits_reject_duplicates_width_and_depth() {
        assert_eq!(
            validate(&[(
                "out",
                Output::Object(&[("src", Output::Null), ("src", Output::Other)])
            )]),
            Err(Error::InvalidOutput)
        );
        let values = vec![Output::Null; MAX_VALUE_NODES];
        assert_eq!(
            validate(&[("out", Output::Array(&values))]),
            Err(Error::Capacity)
        );
        // Each level borrows the enclosing stack frame to build a deep chain without leaking heap memory in the test.
        fn chain(depth: usize, value: Output<'_>) {
            let next = [value];
            if depth == 0 {
                assert_eq!(
                    validate(&[("out", Output::Array(&next))]),
                    Err(Error::Capacity)
                );
            } else {
                chain(depth - 1, Output::Array(&next));
            }
        }
        chain(MAX_VALUE_DEPTH, Output::Null);
        assert!(validate(&[("out", Output::Image("borrowed"))]).is_ok());
    }
}
