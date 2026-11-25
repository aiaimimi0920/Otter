//! 实际完整候选的 sender 索引；只有 canonical checkpoint owner 在 commit 后才能提升证明。
use super::*;
use crate::{
    projection_delivery::checkpoint_session::{self, Object},
    session_source::Parsed,
};
#[derive(Clone)]
pub struct Index {
    source: Source,
    entries: Vec<Binding>,
}

impl Index {
    pub fn capture(source: &Source) -> Option<Self> {
        let parsed = source.parse().ok()?;
        let scope = parsed
            .document()
            .projection_scope
            .map(|scope| scope.to_string());
        let mut index = Self::new(&parsed);
        checkpoint_session::visit(&parsed, |object| index.add(object, scope.as_deref()))?;
        Some(index)
    }
    pub(in crate::projection_delivery) fn new(parsed: &Parsed<'_>) -> Self {
        Self {
            source: parsed.source().clone(),
            entries: Vec::new(),
        }
    }
    pub(in crate::projection_delivery) fn add(
        &mut self,
        object: &Object<'_>,
        scope: Option<&str>,
    ) -> Option<()> {
        let unit: std::borrow::Cow<'_, str> = serde_json::from_str(object.get("id")?.get()).ok()?;
        if let Some(binding) = invitation::capture(object, &unit, scope).ok()? {
            self.push(binding)?;
        }
        let Some(raw) = object
            .get("projectionSenders")
            .filter(|raw| raw.get() != "null")
        else {
            return Some(());
        };
        let bindings: inventory::Entries<'_> = serde_json::from_str(raw.get()).ok()?;
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
            if inventory::native(raw.get()).ok()? {
                self.push(Binding::decode(raw.get(), &unit, scope?).ok()?)?;
            }
        }
        inventory::primary(object, &unit, &keys).ok()?;
        Some(())
    }
    fn push(&mut self, binding: Binding) -> Option<()> {
        if self.entries.len() == MAX_ENTRIES
            || self.entries.iter().any(|old| {
                old.envelope().origin() == binding.envelope().origin()
                    && old.envelope().projection_id() == binding.envelope().projection_id()
            })
        {
            return None;
        }
        self.entries.push(binding);
        Some(())
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
