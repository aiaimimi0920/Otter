//! 表单提交快照：与前端相同的字段数和 UTF-8 总预算，不改变控件可见草稿。
use serde_json::{Map, Value};

const MAX_FIELDS: usize = 16;
const MAX_BYTES: usize = 48 * 1024;
#[derive(Default)]
pub struct Values {
    entries: Map<String, Value>,
    bytes: usize,
}
impl Values {
    /// 按首次登记顺序占用名额；场景移除字段不清除值，attachment 换绑才重置。
    pub fn update(&mut self, id: &str, value: &str) {
        let previous = self.entries.get(id).and_then(Value::as_str);
        if previous.is_none() && self.entries.len() >= MAX_FIELDS {
            return;
        }
        let other = self.bytes - previous.map_or(0, str::len);
        let available = MAX_BYTES - other;
        let mut end = value.len().min(available);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        // UTF-8 的 48KiB 上限也保证不超过前端 64Ki UTF-16 单元上限。
        let bounded = &value[..end];
        if previous != Some(bounded) {
            self.entries
                .insert(id.to_owned(), Value::String(bounded.to_owned()));
        }
        self.bytes = other + end;
    }
    pub fn entries(&self) -> &Map<String, Value> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_values_budget_is_shared_and_truncates_at_utf8_boundary() {
        let mut values = Values::default();
        values.update("a", &"a".repeat(MAX_BYTES - 4));
        values.update("b", "🙂中文");
        assert_eq!(values.entries()["b"], "🙂");
        values.update("a", "short");
        assert_eq!(values.entries()["b"], "🙂");
        values.update("b", "🙂中文");
        assert_eq!(values.entries()["b"], "🙂中文");
        assert_eq!(values.bytes, "short🙂中文".len());
    }
    #[test]
    fn surface_values_field_cap_still_allows_existing_field_updates() {
        let mut values = Values::default();
        for n in 0..17 {
            values.update(&n.to_string(), "x");
        }
        assert_eq!(values.entries().len(), 16);
        assert!(!values.entries().contains_key("16"));
        values.update("0", "changed");
        assert_eq!(values.entries()["0"], "changed");
    }
}
