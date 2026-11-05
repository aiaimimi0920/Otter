//! 只借用协议值，不执行脚本或访问图片路径；包装字段顺序与主干 getObjectField 一致。
use super::Error;
pub const MAX_VALUE_NODES: usize = 4096;
pub const MAX_VALUE_DEPTH: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Output<'a> {
    /// 字符串输出（保留旧命名），是否为可用图片引用由消费端判断。
    Image(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
    Object(&'a [(&'a str, Output<'a>)]),
    Array(&'a [Output<'a>]),
    /// 已解码 JSON 容器的借用视图；不为图解析复制整棵输出树。
    Json(&'a serde_json::Value),
    /// 历史无可序列化值哨兵；JSON 对象字段省略，数组位置编码 null。
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
    /// 只解包一层；false、0、空字符串是存在的值，不能跳过它们去取后面的 src。
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
/// JSON 解码适配器仍须限制输入字节；这里限制解析图视图中的结构遍历工作量。
pub(crate) fn validate(values: &[(&str, Output<'_>)]) -> Result<(), Error> {
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
        // 每层借用上层栈帧构建深链，测试不泄漏堆内存。
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
