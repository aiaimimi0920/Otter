//! 实际完整候选的 sender 索引；只有 canonical checkpoint owner 在 commit 后才能提升证明。
use super::*;
use serde_json::value::RawValue;
#[derive(Clone)]
pub struct Index {
    source: Source,
    entries: Vec<Binding>,
}
#[derive(Deserialize)]
struct Windows<'a>(#[serde(borrow, deserialize_with = "windows")] Vec<&'a RawValue>);
fn windows<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, { crate::workspace::MAX_UNITS }>(decoder)
}
impl Index {
    pub fn capture(source: &Source) -> Option<Self> {
        let document = crate::session_document::decode(source).ok()?;
        let scope = document.projection_scope.map(|scope| scope.to_string());
        let root = fields(std::str::from_utf8(source).ok()?).ok()?;
        let windows: Windows<'_> = serde_json::from_str(root.get("stickers")?.get()).ok()?;
        let mut entries: Vec<Binding> = Vec::new();
        for window in windows.0 {
            let object = fields(window.get()).ok()?;
            let Some(raw) = object.get("projectionSenders") else {
                continue;
            };
            if raw.get() == "null" {
                continue;
            } // 旧 null 保留，但绝不成为原生关联。
            let bindings: inventory::Entries<'_> = serde_json::from_str(raw.get()).ok()?;
            let unit: std::borrow::Cow<'_, str> =
                serde_json::from_str(object.get("id")?.get()).ok()?;
            let mut keys: Vec<inventory::Key> = Vec::new();
            for raw in bindings.0 {
                let key = inventory::key(raw.get(), &unit).ok()?;
                if keys.iter().any(|old| {
                    (old.origin == key.origin && old.projection == key.projection)
                        || old.destination == key.destination
                }) {
                    return None;
                }
                keys.push(key);
                if !inventory::native(raw.get()).ok()? {
                    continue;
                }
                let binding = Binding::decode(raw.get(), &unit, scope.as_deref()?).ok()?;
                if entries.len() == MAX_ENTRIES
                    || entries.iter().any(|old| {
                        old.envelope().origin() == binding.envelope().origin()
                            && old.envelope().projection_id() == binding.envelope().projection_id()
                    })
                {
                    return None;
                }
                entries.push(binding);
            }
            inventory::primary(&object, &unit, &keys).ok()?;
        }
        Some(Self {
            source: source.clone(),
            entries,
        })
    }
    pub fn is_for(&self, source: &Source) -> bool {
        self.source.same_storage(source)
    }
    pub fn includes(&self, binding: &Binding) -> bool {
        self.entries.iter().any(|entry| entry == binding)
    }
    pub fn for_intent(&self, intent: &Intent) -> Option<&Binding> {
        self.entries
            .iter()
            .find(|entry| entry.matches_intent(intent))
    }
    pub fn entries(&self) -> &[Binding] {
        &self.entries
    }
    pub fn for_unit<'a>(&'a self, unit: &'a str) -> impl Iterator<Item = &'a Binding> {
        self.entries
            .iter()
            .filter(move |entry| entry.unit() == unit)
    }
}
