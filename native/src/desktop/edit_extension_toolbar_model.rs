//! 工具贡献的主干分组与顺序；成功目录/文档凭证缓存，不在 paint 或拖动热路径解析 payload。
use super::*;
use crate::loom_extension_snapshot::ToolbarItem;
pub(super) struct Slot {
    pub id: String,
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
        // OCR 已有主按钮及全文复制；其它插件贡献不得硬编码为产品按钮。
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
            let id = format!(
                "{}:{}",
                item.plugin,
                item.group.as_deref().unwrap_or(&item.id)
            );
            if let Some(slot) = slots.iter_mut().find(|s| s.id == id) {
                slot.items.push(item);
                continue;
            }
            slots.push(Slot {
                id,
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
            .map(|s| (s.id.encode_utf16().collect::<Vec<_>>(), s))
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
    // 控件只接收单行可见文本；不改变模型中的精确 commandId 或授权数据。
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}
