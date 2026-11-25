//! Baseline grouping/order for tool contributions; cache successful catalog/document evidence without parsing payloads in paint or drag hot paths.
use super::*;
use crate::loom_extension_snapshot::ToolbarItem;
pub(super) struct Slot {
    pub sort_key: String,
    pub title: String,
    pub grouped: bool,
    pub items: Vec<ToolbarItem>,
}
pub(super) struct Model {
    pub catalog: Arc<Snapshot>,
    pub stamp: extension_document::Stamp,
    pub context: Context,
    pub slots: Vec<Slot>,
    pub token: u32,
}
impl Model {
    pub unsafe fn current(&self, state: &State) -> bool {
        self.stamp.matches(&state.extension_document.borrow())
            && self.context == Context::capture(state)
            && state
                .extensions
                .borrow()
                .snapshot
                .as_ref()
                .is_some_and(|s| Arc::ptr_eq(s, &self.catalog))
    }
    pub unsafe fn capture(state: &State, catalog: Arc<Snapshot>, token: u32) -> Result<Self> {
        let context = Context::capture(state);
        let mut items = context.with(|c| catalog.toolbar(c));
        // OCR already has a primary button/full-text copy; do not hardcode other plugin contributions as product buttons.
        items.retain(|i| i.plugin != "neuro.official/ocr");
        let collator =
            super::super::text_collation::Collator::new(windows::core::s!("en_US"), &[])?;
        let mut ordered: Vec<_> = items
            .into_iter()
            .map(|i| (i.id.encode_utf16().collect::<Vec<_>>(), i))
            .collect();
        ordered
            .sort_by(|(a, x), (b, y)| x.order.cmp(&y.order).then_with(|| collator.compare(a, b)));
        let mut slots: Vec<Slot> = Vec::new();
        for (_, item) in ordered {
            let sort_key = format!(
                "{}:{}",
                item.plugin,
                item.group.as_deref().unwrap_or(&item.id)
            );
            // ID and groupId may match and contain delimiters; merge only contributions from the exact same plugin/group.
            if let Some(slot) = slots.iter_mut().find(|slot| {
                item.group.is_some()
                    && slot.items[0].plugin == item.plugin
                    && slot.items[0].group == item.group
            }) {
                slot.items.push(item);
                continue;
            }
            slots.push(Slot {
                sort_key,
                title: item
                    .group_title
                    .as_ref()
                    .filter(|_| item.group.is_some())
                    .unwrap_or(&item.title)
                    .clone(),
                grouped: item.group.is_some(),
                items: vec![item],
            });
        }
        let mut ordered: Vec<_> = slots
            .into_iter()
            .map(|s| (s.sort_key.encode_utf16().collect::<Vec<_>>(), s))
            .collect();
        ordered.sort_by(|(a, x), (b, y)| {
            x.items[0]
                .order
                .cmp(&y.items[0].order)
                .then_with(|| collator.compare(a, b))
        });
        Ok(Self {
            catalog,
            context,
            token,
            stamp: state.extension_document.borrow().stamp(),
            slots: ordered.into_iter().take(8).map(|(_, s)| s).collect(),
        })
    }
}
pub(super) fn text(value: &str, max: usize) -> String {
    // Controls receive only single-line visible text; preserve exact model commandId and authorization data.
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}
