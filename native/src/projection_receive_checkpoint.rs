//! 实际候选字节的有界关联索引；自身仍不是提交证明，只能由 checkpoint 发布者提升。
use super::{Binding, Ticket};
use crate::{
    projection_delivery::checkpoint_session::{self, Object},
    session_json_fields::fields,
    session_source::{Parsed, Source},
};

#[derive(Clone)]
pub struct Index {
    source: Source,
    entries: Vec<Binding>,
}
impl Index {
    /// 只在原准备 worker 中解析；未知/旧投送不推断关联，坏标记不阻止普通工作区保存。
    pub(crate) fn capture(source: &Source) -> Option<Self> {
        let parsed = source.parse().ok()?;
        let mut index = Self::new(&parsed);
        checkpoint_session::visit(&parsed, |object| index.add(object))?;
        Some(index)
    }
    pub(in crate::projection_delivery) fn new(parsed: &Parsed<'_>) -> Self {
        Self {
            source: parsed.source().clone(),
            entries: Vec::new(),
        }
    }
    pub(in crate::projection_delivery) fn add(&mut self, object: &Object<'_>) -> Option<()> {
        let Some(raw) = object.get("qrProjection").filter(|raw| raw.get() != "null") else {
            return Some(());
        };
        // 只借用大型图片字段；投送元数据本身有独立小上限。
        if raw.get().len() > 8192 {
            return None;
        }
        let projection = fields(raw.get()).ok()?;
        if !projection.contains_key("otterNativeReceive") {
            return Some(());
        }
        let binding: Binding = serde_json::from_str(raw.get()).ok()?;
        let unit: &str = serde_json::from_str(object.get("id")?.get()).ok()?;
        if !binding.valid(unit)
            || self.entries.len() == super::super::journal::MAX_ENTRIES
            || self.entries.iter().any(|old| {
                old.otter_native_receive.intent_id == binding.otter_native_receive.intent_id
            })
        {
            return None;
        }
        self.entries.push(binding);
        Some(())
    }
    pub(crate) fn is_for(&self, source: &Source) -> bool {
        self.source.same_storage(source)
    }
    pub fn for_ticket(&self, ticket: &Ticket) -> Option<&Binding> {
        self.entries
            .iter()
            .find(|entry| entry.matches_ticket(ticket))
    }
    pub fn includes(&self, expected: &Binding) -> bool {
        self.entries.iter().any(|entry| entry == expected)
    }
    pub(crate) fn for_unit(&self, unit: &str) -> Option<&Binding> {
        self.entries.iter().find(|entry| entry.unit() == unit)
    }
}
