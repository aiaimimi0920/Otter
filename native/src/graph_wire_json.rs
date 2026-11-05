//! 有界 JSON 输出器；只编码借用协议值，不执行对象方法、不访问外部资源。
use super::{Error, MAX_MESSAGE_BYTES};
use crate::graph_image::Output;
pub(crate) struct Json(pub String, usize);
#[cfg(test)]
#[test]
fn borrowed_json_containers_encode_with_existing_byte_and_depth_limits() {
    let value = serde_json::json!({"文本":[null,false,0,{"value":"a\nb"}]});
    let mut encoded = Json::new();
    encoded.value(Output::from_json(&value), 0).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&encoded.0).unwrap(),
        value
    );
    assert!(Json::with_limit(8)
        .value(Output::from_json(&value), 0)
        .is_err());
    assert!(Json::new().value(Output::from_json(&value), 33).is_err());
}
impl Json {
    pub fn new() -> Self {
        Self(String::new(), MAX_MESSAGE_BYTES)
    }
    pub fn with_limit(limit: usize) -> Self {
        Self(String::new(), limit)
    }
    pub fn raw(&mut self, text: &str) -> Result<(), Error> {
        if self
            .0
            .len()
            .checked_add(text.len())
            .is_none_or(|length| length > self.1)
        {
            return Err(Error::MessageTooLarge);
        }
        self.0
            .try_reserve(text.len())
            .map_err(|_| Error::Allocation)?;
        self.0.push_str(text);
        Ok(())
    }
    pub fn string(&mut self, text: &str) -> Result<(), Error> {
        if text
            .len()
            .checked_add(2)
            .is_none_or(|length| length > self.1 - self.0.len())
        {
            return Err(Error::MessageTooLarge);
        }
        self.raw("\"")?;
        let mut start = 0;
        for (index, character) in text.char_indices() {
            let escaped = match character {
                '"' => Some("\\\""),
                '\\' => Some("\\\\"),
                '\n' => Some("\\n"),
                '\r' => Some("\\r"),
                '\t' => Some("\\t"),
                '\u{0008}' => Some("\\b"),
                '\u{000c}' => Some("\\f"),
                _ => None,
            };
            if let Some(escaped) = escaped {
                self.raw(&text[start..index])?;
                self.raw(escaped)?;
            } else if character < '\u{0020}' {
                self.raw(&text[start..index])?;
                const HEX: &[u8] = b"0123456789abcdef";
                let code = character as usize;
                let escaped = [b'\\', b'u', b'0', b'0', HEX[code >> 4], HEX[code & 15]];
                // 全部字节来自 ASCII 常量，不为每个控制字符分配格式化字符串。
                self.raw(std::str::from_utf8(&escaped).unwrap())?;
            } else {
                continue;
            }
            start = index + character.len_utf8();
        }
        self.raw(&text[start..])?;
        self.raw("\"")
    }
    pub fn value(&mut self, value: Output<'_>, depth: usize) -> Result<(), Error> {
        if depth > 32 {
            return Err(Error::InvalidValue);
        }
        match value {
            Output::Json(value) => match value {
                serde_json::Value::Array(values) => {
                    self.raw("[")?;
                    for (index, value) in values.iter().enumerate() {
                        if index != 0 {
                            self.raw(",")?;
                        }
                        self.value(Output::from_json(value), depth + 1)?;
                    }
                    self.raw("]")
                }
                serde_json::Value::Object(fields) => {
                    self.raw("{")?;
                    let mut first = true;
                    for (name, value) in fields {
                        self.field(name, &mut first)?;
                        self.value(Output::from_json(value), depth + 1)?;
                    }
                    self.raw("}")
                }
                _ => self.value(Output::from_json(value), depth),
            },
            Output::Image(value) => self.string(value),
            Output::Number(value) if value.is_finite() => self.raw(&if value == 0.0 {
                "0".into()
            } else {
                value.to_string()
            }),
            Output::Number(_) | Output::Null | Output::Other => self.raw("null"),
            Output::Boolean(value) => self.raw(if value { "true" } else { "false" }),
            Output::Array(values) => {
                self.raw("[")?;
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        self.raw(",")?;
                    }
                    self.value(*value, depth + 1)?;
                }
                self.raw("]")
            }
            Output::Object(fields) => {
                self.raw("{")?;
                let mut first = true;
                for (name, value) in fields {
                    if matches!(value, Output::Other) {
                        continue;
                    }
                    self.field(name, &mut first)?;
                    self.value(*value, depth + 1)?;
                }
                self.raw("}")
            }
        }
    }
    pub fn field(&mut self, name: &str, first: &mut bool) -> Result<(), Error> {
        if !*first {
            self.raw(",")?;
        }
        *first = false;
        self.string(name)?;
        self.raw(":")
    }
}
