//! 实际候选字节的有界关联索引；自身仍不是提交证明，只能由 checkpoint 发布者提升。
use super::{Binding, Ticket};
use crate::{session_document, session_json_fields::fields};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

#[derive(Clone)]
pub struct Index {
    source: crate::session_source::Source,
    entries: Vec<Binding>,
}
#[derive(Deserialize)]
struct Stickers<'a>(#[serde(borrow, deserialize_with = "stickers")] Vec<&'a RawValue>);
fn stickers<'de: 'a, 'a, D: Deserializer<'de>>(decoder: D) -> Result<Vec<&'a RawValue>, D::Error> {
    session_document::bounded::<D, &'a RawValue, { crate::workspace::MAX_UNITS }>(decoder)
}
impl Index {
    /// 只在原准备 worker 中解析；未知/旧投送不推断关联，坏标记不阻止普通工作区保存。
    pub(crate) fn capture(source: &crate::session_source::Source) -> Option<Self> {
        let bytes: &[u8] = source;
        session_document::decode(bytes).ok()?;
        let root = fields(std::str::from_utf8(bytes).ok()?).ok()?;
        let raw: Stickers<'_> = serde_json::from_str(root.get("stickers")?.get()).ok()?;
        let mut entries = Vec::<Binding>::new();
        for sticker in raw.0 {
            let object = fields(sticker.get()).ok()?;
            let Some(raw) = object.get("qrProjection") else {
                continue;
            };
            if raw.get() == "null" {
                continue; // 旧会话普通贴纸会显式写 null，不能因此屏蔽其他合法的原生关联。
            }
            // 只借用大型图片字段；投送元数据本身有独立小上限，避免未知字段洪泛。
            if raw.get().len() > 8192 {
                return None;
            }
            let projection = fields(raw.get()).ok()?;
            if !projection.contains_key("otterNativeReceive") {
                continue;
            }
            let binding: Binding = serde_json::from_str(raw.get()).ok()?;
            let unit: &str = serde_json::from_str(object.get("id")?.get()).ok()?;
            if !binding.valid(unit)
                || entries.len() == super::super::journal::MAX_ENTRIES
                || entries.iter().any(|old| {
                    old.otter_native_receive.intent_id == binding.otter_native_receive.intent_id
                })
            {
                return None;
            }
            entries.push(binding);
        }
        Some(Self {
            source: source.clone(),
            entries,
        })
    }
    pub(crate) fn is_for(&self, source: &crate::session_source::Source) -> bool {
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
