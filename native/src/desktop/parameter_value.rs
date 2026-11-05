//! 原生单参数输入合同；解析公共值，不读取文件、执行脚本或接触 secret。
use crate::loom_capability::Parameter;
use serde_json::Value;
pub(super) const MAX_TEXT: usize = 16384;
#[derive(Clone, PartialEq)]
pub(super) struct Spec {
    pub title: String,
    pub multiline: bool,
    pub number: bool,
    json: bool,
    step: f64,
    min: Option<f64>,
    max: Option<f64>,
}
impl Spec {
    pub fn from_parameter(parameter: &Parameter<'_>) -> Option<Self> {
        if parameter.secret
            || !matches!(
                parameter.widget.as_str(),
                "number" | "slider" | "text" | "textarea" | "path"
            )
        {
            return None;
        }
        let number = matches!(parameter.widget.as_str(), "number" | "slider");
        let json = !number
            && parameter.data_type.is_some_and(|kind| {
                matches!(
                    kind.to_ascii_lowercase().as_str(),
                    "json" | "object" | "array"
                )
            });
        Some(Self {
            title: parameter
                .label
                .chars()
                .filter(|c| !c.is_control())
                .take(256)
                .collect(),
            multiline: parameter.multiline,
            number,
            json,
            step: parameter.step.filter(|value| *value > 0.0).unwrap_or(1.0),
            min: parameter.min,
            max: parameter.max,
        })
    }
    pub fn initial(&self, value: Option<&Value>) -> Option<String> {
        if self.number {
            let number =
                super::parameter_display_value::number(value).unwrap_or(self.min.unwrap_or(0.0));
            return self.number_value(number).map(|value| value.to_string());
        }
        let text = match value {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(text)) if !self.number => text.clone(),
            Some(value) if self.json => serde_json::to_string_pretty(value).ok()?,
            _ => super::parameter_display_value::text(value),
        };
        (text.encode_utf16().count() <= MAX_TEXT).then_some(text)
    }
    pub fn parse(&self, text: &str) -> Option<Value> {
        if text.encode_utf16().count() > MAX_TEXT {
            return None;
        }
        if !self.number {
            if self.json {
                return serde_json::from_str(text).ok();
            }
            return Some(Value::String(text.into()));
        }
        self.number_value(super::parameter_number::parse(text)?)
    }
    fn number_value(&self, number: f64) -> Option<Value> {
        let number = super::parameter_number::normalize(number, self.min, self.max)?;
        if number.fract() == 0.0
            && (-9223372036854775808.0..9223372036854775808.0).contains(&number)
        {
            Some(Value::from(number as i64))
        } else {
            serde_json::Number::from_f64(number).map(Value::Number)
        }
    }
    pub fn stepped(&self, text: &str, up: bool) -> Option<String> {
        if !self.number {
            return None;
        }
        let current = self.parse(text)?.as_f64()?;
        Some(
            self.number_value(current + if up { self.step } else { -self.step })?
                .to_string(),
        )
    }
    pub fn slider(&self, text: &str) -> Option<super::parameter_slider::Range> {
        if !self.number {
            return None;
        }
        super::parameter_slider::Range::new(
            self.parse(text)?.as_f64()?,
            self.min,
            self.max,
            self.step,
        )
    }
    pub fn hint(&self) -> String {
        if self.number {
            format!(
                "{}；最小值 {}，最大值 {}",
                "请输入数字（越界值限制到范围内）",
                self.min.map_or("不限".into(), |n| n.to_string()),
                self.max.map_or("不限".into(), |n| n.to_string())
            )
        } else if self.json {
            format!("请输入有效 JSON，最多 {} 个 UTF-16 单元", MAX_TEXT)
        } else {
            format!("最多 {} 个 UTF-16 单元", MAX_TEXT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(value: Value) -> Spec {
        let raw = serde_json::json!({"id":"art","parameters":[value]});
        let result = Spec::from_parameter(
            &crate::loom_capability::Capability::new(&raw)
                .unwrap()
                .parameters()
                .next()
                .unwrap(),
        )
        .unwrap();
        result
    }
    #[test]
    fn numbers_preserve_scalar_type_and_enforce_declared_bounds() {
        let spec = spec(serde_json::json!({"id":"n","type":"integer","min":-2,"max":10}));
        assert_eq!(spec.parse(" 3 "), Some(serde_json::json!(3)));
        for text in ["NaN", "1e999", "true", "\"3\"", "null"] {
            assert!(spec.parse(text).is_none(), "{text}");
        }
        assert_eq!(spec.initial(Some(&Value::Bool(true))).as_deref(), Some("1"));
        assert_eq!(spec.parse("11"), Some(serde_json::json!(10)));
        assert_eq!(spec.parse("-3"), Some(serde_json::json!(-2)));
        assert_eq!(spec.parse("3.5"), Some(serde_json::json!(3.5)));
        assert_eq!(spec.parse(""), Some(serde_json::json!(0)));
        assert_eq!(spec.stepped("10", true).as_deref(), Some("10"));
    }
    #[test]
    fn strings_preserve_whitespace_and_optional_empty_values() {
        let text = spec(serde_json::json!({"id":"text","type":"string","multiline":true}));
        assert!(text.multiline);
        assert_eq!(
            text.parse(" \n中文 "),
            Some(Value::String(" \n中文 ".into()))
        );
        assert_eq!(text.parse(""), Some(Value::String(String::new())));
        assert!(text.parse(&"a".repeat(MAX_TEXT + 1)).is_none());
        let required = spec(serde_json::json!({"id":"text","required":true}));
        assert_eq!(required.parse(""), Some(Value::String(String::new())));
    }
    #[test]
    fn decimal_steps_and_widget_aliases_use_the_same_input_contract() {
        let slider =
            spec(serde_json::json!({"id":"n","widget":"slider","step":0.1,"min":0,"max":1}));
        assert_eq!(slider.stepped("0.2", true).as_deref(), Some("0.3"));
        assert_eq!(slider.stepped("0", false).as_deref(), Some("0"));
        let invalid_step = spec(serde_json::json!({"id":"n","type":"number","step":0}));
        assert_eq!(invalid_step.stepped("2", true).as_deref(), Some("3"));
        let path = spec(serde_json::json!({"id":"path","widget":"path"}));
        assert_eq!(
            path.parse("C:/not-opened"),
            Some(Value::String("C:/not-opened".into()))
        );
    }
    #[test]
    fn structured_input_matches_main_formatting_and_accepts_any_json_value() {
        for kind in ["json", "object", "ARRAY"] {
            let field = spec(serde_json::json!({"id":"payload","type":kind}));
            let value = serde_json::json!({"items":[1,true,null],"text":"中文"});
            let text = field.initial(Some(&value)).unwrap();
            assert!(text.contains('\n'));
            assert_eq!(field.parse(&text), Some(value));
            for value in [
                serde_json::json!(true),
                serde_json::json!(42),
                serde_json::json!(null),
                serde_json::json!([1, 2]),
            ] {
                assert_eq!(field.parse(&value.to_string()), Some(value));
            }
            // 主干字符串原样显示，不自动包引号；null/未设置显示空文本。
            assert_eq!(
                field.initial(Some(&Value::String("raw".into()))).as_deref(),
                Some("raw")
            );
            assert_eq!(field.initial(Some(&Value::Null)).as_deref(), Some(""));
            for invalid in ["raw", "", "{", "[1,]", "NaN"] {
                assert!(field.parse(invalid).is_none());
            }
            assert!(field
                .parse(&format!("\"{}\"", "a".repeat(MAX_TEXT)))
                .is_none());
        }
    }
}
