//! 附件的同主干 JSON 预算和规范摘要；排序按 UTF-16，保证跨前端/原生的来源凭证一致。
use serde_json::Value;

pub(crate) fn bounded(value: &Value, bytes: usize) -> Result<(), ()> {
    fn visit(value: &Value, depth: usize, nodes: &mut usize, max: usize) -> Result<(), ()> {
        *nodes += 1;
        if depth > 32 || *nodes > max {
            return Err(());
        }
        match value {
            Value::Array(items) => {
                for item in items {
                    visit(item, depth + 1, nodes, max)?;
                }
            }
            Value::Object(items) => {
                for item in items.values() {
                    visit(item, depth + 1, nodes, max)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
    visit(value, 0, &mut 0, bytes.div_ceil(2))?;
    if serde_json::to_vec(value).map_err(|_| ())?.len() > bytes {
        return Err(());
    }
    Ok(())
}

// serde_json 的最短 IEEE-754 数字采用相同的舍入规则；这里仅投影 ECMAScript 的
// 十进制/指数阈值、正指数符号和 -0。整数也先经过 JS Number 的 f64 语义。
fn number(value: &serde_json::Number) -> String {
    let value = value.as_f64().expect("finite JSON number");
    if value == 0.0 {
        return "0".into();
    }
    let raw = serde_json::to_string(&value.abs()).expect("finite JSON number");
    let (mantissa, exponent) = raw.split_once('e').map_or((raw.as_str(), 0), |(m, e)| {
        (m, e.parse::<i32>().expect("serializer exponent"))
    });
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as i32 + exponent;
    let mut digits = mantissa.replace('.', "");
    while digits.ends_with('0') {
        digits.pop();
    }
    let mut result = if value < 0.0 {
        "-".to_owned()
    } else {
        String::new()
    };
    if decimal > 0 && decimal <= 21 {
        let split = (decimal as usize).min(digits.len());
        result.push_str(&digits[..split]);
        if split < digits.len() {
            result.push('.');
            result.push_str(&digits[split..]);
        } else {
            result.extend(std::iter::repeat_n('0', decimal as usize - digits.len()));
        }
    } else if decimal <= 0 && decimal > -6 {
        result.push_str("0.");
        result.extend(std::iter::repeat_n('0', (-decimal) as usize));
        result.push_str(&digits);
    } else {
        result.push_str(&digits[..1]);
        if digits.len() > 1 {
            result.push('.');
            result.push_str(&digits[1..]);
        }
        result.push('e');
        if decimal > 0 {
            result.push('+');
        }
        result.push_str(&(decimal - 1).to_string());
    }
    result
}
pub(crate) fn canonical(value: &Value) -> Vec<u8> {
    fn write(value: &Value, out: &mut Vec<u8>) {
        match value {
            Value::Array(items) => {
                out.push(b'[');
                for (index, item) in items.iter().enumerate() {
                    if index != 0 {
                        out.push(b',');
                    }
                    write(item, out);
                }
                out.push(b']');
            }
            Value::Object(items) => {
                let mut keys: Vec<_> = items.keys().collect();
                keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
                out.push(b'{');
                for (index, key) in keys.iter().enumerate() {
                    if index != 0 {
                        out.push(b',');
                    }
                    serde_json::to_writer(&mut *out, key)
                        .expect("JSON object keys are serializable");
                    out.push(b':');
                    write(&items[*key], out);
                }
                out.push(b'}');
            }
            Value::Number(value) => out.extend_from_slice(number(value).as_bytes()),
            _ => serde_json::to_writer(out, value).expect("validated JSON values are serializable"),
        }
    }
    let mut bytes = Vec::new();
    write(value, &mut bytes);
    bytes
}
pub(crate) fn digest(value: &Value) -> String {
    ring::digest::digest(&ring::digest::SHA256, &canonical(value))
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[cfg(test)]
#[path = "unit_attachment_json_tests.rs"]
mod tests;
