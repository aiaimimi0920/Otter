//! 主干 Art VIEW 声明的只读投影；呈现元数据不是包证明或执行授权。
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Id(String);

impl Id {
    pub fn parse(value: &str) -> Option<Self> {
        valid_id(value).then(|| Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| serde::de::Error::custom("Invalid Art surface view ID"))
    }
}

fn valid_id(value: &str) -> bool {
    (1..=160).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub width: u32,
    pub height: u32,
}

/// 借用已接纳的目录，不复制 metadata；调用者仍须保有目录/来源身份租约。
pub struct Views<'a> {
    entries: Vec<View<'a>>,
    default: Option<&'a str>,
}

// 与 ECMAScript String.trim 一致；Rust trim 会遗漏 FEFF，却额外去掉 0085。
fn whitespace(value: char) -> bool {
    matches!(value, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn dimension(value: &Value) -> Option<u32> {
    let number = value.as_f64()?;
    // JSON 的 100 和 100.0 都是主干接受的整数；不四舍五入无效的尺寸。
    ((1.0..=16384.0).contains(&number) && number.fract() == 0.0).then_some(number as u32)
}

fn view(value: &Value) -> Option<View<'_>> {
    let id = value.get("id")?.as_str().filter(|value| valid_id(value))?;
    let label = value.get("label")?.as_str()?;
    if label.trim_matches(whitespace).is_empty() || label.encode_utf16().count() > 80 {
        return None;
    }
    let size = value.get("fullSize")?;
    Some(View {
        id,
        label,
        width: dimension(size.get("width")?)?,
        height: dimension(size.get("height")?)?,
    })
}

impl<'a> Views<'a> {
    pub fn from_manifest(surface: Option<&'a Value>) -> Self {
        // 上游目录有整体字节预算；这里线性过滤并保留首个有效同 ID 声明。
        let mut seen = HashSet::new();
        let entries = surface
            .and_then(|value| value.get("views"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(view)
            .filter(|view| seen.insert(view.id))
            .collect();
        Self {
            entries,
            default: surface
                .and_then(|value| value.get("defaultViewId"))
                .and_then(Value::as_str),
        }
    }
    pub fn entries(&self) -> &[View<'a>] {
        &self.entries
    }
    pub fn resolve(&self, requested: Option<&str>) -> Option<&View<'a>> {
        self.entries
            .iter()
            .find(|view| Some(view.id) == requested)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|view| Some(view.id) == self.default)
            })
            .or_else(|| self.entries.first())
    }
}

#[cfg(test)]
#[path = "art_surface_views_tests.rs"]
mod tests;
