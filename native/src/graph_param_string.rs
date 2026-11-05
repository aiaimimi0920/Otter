//! JS String 的协议值子集；数组 join 的空元素/嵌套规则与原转换保持一致。
use super::{Output, MAX_TEXT_BYTES};
pub(super) fn convert(value: Output<'_>) -> Option<String> {
    let mut text = String::new();
    append(&mut text, value, 0, false)?;
    Some(text)
}
fn push(text: &mut String, value: &str) -> Option<()> {
    if text.len().checked_add(value.len())? > MAX_TEXT_BYTES {
        return None;
    }
    text.try_reserve(value.len()).ok()?;
    text.push_str(value);
    Some(())
}
fn append(text: &mut String, value: Output<'_>, depth: usize, in_array: bool) -> Option<()> {
    if depth > 32 {
        return None;
    }
    match value {
        Output::Json(value) => match value {
            serde_json::Value::Object(_) => push(text, "[object Object]"),
            serde_json::Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        push(text, ",")?;
                    }
                    append(text, Output::from_json(value), depth + 1, true)?;
                }
                Some(())
            }
            _ => append(text, Output::from_json(value), depth, in_array),
        },
        Output::Image(value) => push(text, value),
        Output::Null if in_array => Some(()),
        Output::Null => push(text, "null"),
        Output::Other if in_array => Some(()),
        Output::Other => push(text, "undefined"),
        Output::Boolean(value) => push(text, if value { "true" } else { "false" }),
        Output::Object(_) => push(text, "[object Object]"),
        Output::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    push(text, ",")?;
                }
                append(text, *value, depth + 1, true)?;
            }
            Some(())
        }
        Output::Number(value) => {
            if value.is_nan() {
                return push(text, "NaN");
            }
            if value.is_infinite() {
                return push(
                    text,
                    if value.is_sign_negative() {
                        "-Infinity"
                    } else {
                        "Infinity"
                    },
                );
            }
            if value == 0.0 {
                return push(text, "0");
            }
            let repr = if value.abs() >= 1e21 || value.abs() < 1e-6 {
                let scientific = format!("{value:e}");
                let (mantissa, exponent) = scientific.split_once('e')?;
                let exponent = exponent.parse::<i32>().ok()?;
                format!("{mantissa}e{exponent:+}")
            } else {
                value.to_string()
            };
            push(text, &repr)
        }
    }
}
