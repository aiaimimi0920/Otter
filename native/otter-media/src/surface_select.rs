//! Select 的有界声明快照；严格字符串与旧前端一致，不创建文本 draft 或表单值。
use crate::surface_scene::{Kind, Node};
use serde_json::Value;
pub const MAX_OPTIONS: usize = 2048;
pub const MAX_TEXT_BYTES: usize = crate::surface_input_draft::MAX_VALUE_BYTES;
pub const MAX_TOTAL_BYTES: usize = crate::surface_scene::MAX_BYTES;
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Kind,
    Limit,
    Text,
}
#[derive(Debug, PartialEq, Eq)]
pub struct OptionItem {
    value: String,
    label: String,
}
impl OptionItem {
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn label(&self) -> &str {
        &self.label
    }
}
pub struct Options {
    source: String,
    items: Vec<OptionItem>,
}
fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("")
}
fn validate(value: &str, total: &mut usize) -> Result<(), Error> {
    if value.contains('\0') {
        return Err(Error::Text);
    }
    *total = total.checked_add(value.len()).ok_or(Error::Limit)?;
    if value.len() > MAX_TEXT_BYTES || *total > MAX_TOTAL_BYTES {
        return Err(Error::Limit);
    }
    Ok(())
}
impl Options {
    pub fn parse(node: &Node) -> Result<Self, Error> {
        if node.kind() != Kind::Select {
            return Err(Error::Kind);
        }
        let source = text(node.props().get("value"));
        let items = node
            .props()
            .get("options")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if items.len() > MAX_OPTIONS {
            return Err(Error::Limit);
        }
        let mut total = 0;
        validate(source, &mut total)?;
        // 全部检查先于复制，拒绝时不返回截断的可见列表或部分候选。
        for item in items {
            validate(text(item.get("value")), &mut total)?;
            validate(text(item.get("label")), &mut total)?;
        }
        Ok(Self {
            source: source.to_owned(),
            items: items
                .iter()
                .map(|item| OptionItem {
                    value: text(item.get("value")).to_owned(),
                    label: text(item.get("label")).to_owned(),
                })
                .collect(),
        })
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn items(&self) -> &[OptionItem] {
        &self.items
    }
    /// 不把未匹配的声明值静默改成第一个选项；重复 value 保留，定位首个匹配项。
    pub fn source_index(&self) -> Option<usize> {
        self.items.iter().position(|item| item.value == self.source)
    }
    /// HWND 索引须经范围检查，不能把 CB_ERR/-1 强转后读入任意值。
    pub fn value_at(&self, index: usize) -> Option<&str> {
        self.items.get(index).map(OptionItem::value)
    }
}
#[cfg(test)]
#[path = "surface_select_tests.rs"]
mod tests;
