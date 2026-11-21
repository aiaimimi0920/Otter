//! 文档附件/源版本 owner；保存只冻结 Arc 值，源内容变化推进版本，显隐不推进源版本。
use super::{canvas::Canvas, display_frame::Frame, State};
use crate::{
    loom_extension_snapshot::MAX_INTEGER,
    unit_attachment_store::{Stamp as StoreStamp, Store},
};
use serde_json::{value::RawValue, Map, Value};
use std::sync::Arc;
pub(super) struct Owner {
    pub(super) store: Store,
    pub(super) legacy_text: crate::sticker_extra_text::Legacy,
    revision: Option<u64>,
    source: Option<Source>,
    identity: Arc<()>,
}
impl Default for Owner {
    fn default() -> Self {
        Self {
            store: Store::default(),
            legacy_text: Default::default(),
            revision: Some(0),
            source: None,
            identity: Arc::new(()),
        }
    }
}
#[derive(Clone)]
pub(super) struct Snapshot {
    pub(super) store: Store,
    revision: Option<u64>,
}
#[derive(Clone)]
pub(super) struct Stamp {
    store: StoreStamp,
    source: Arc<()>,
}
/// 只追踪已提交内容与外部来源；尺寸、透明度、样式和缩放视口不改变识别来源。
pub(super) struct Source {
    document: crate::editor::DocumentStamp,
    display: Option<Arc<()>>,
}
impl Source {
    pub(super) fn capture(canvas: &Canvas, frame: Option<&Frame>) -> Self {
        Self {
            document: canvas.editor.document_stamp(),
            display: frame.map(|f| f.identity.clone()),
        }
    }
    pub(super) fn matches(&self, editor: &crate::editor::Editor, frame: Option<&Frame>) -> bool {
        self.document.matches_content(editor)
            && match (&self.display, frame) {
                (None, None) => true,
                (Some(identity), Some(frame)) => Arc::ptr_eq(identity, &frame.identity),
                _ => false,
            }
    }
}
impl Owner {
    pub(super) fn restore(store: Store, propagation: Option<&RawValue>) -> Self {
        let revision = propagation.map_or(Some(0), |raw| {
            let value: Value = serde_json::from_str(raw.get()).ok()?;
            value
                .as_object()?
                .get("revision")
                .map_or(Some(0), |v| v.as_u64().filter(|v| *v <= MAX_INTEGER))
        });
        Self {
            store,
            legacy_text: Default::default(),
            revision,
            source: None,
            identity: Arc::new(()),
        }
    }
    pub(super) fn observe(&mut self, canvas: &Canvas, frame: Option<&Frame>) {
        // 草稿不是已提交源；不能在每个鼠标 sample 推进持久化来源版本。
        if canvas.editor.draft().is_some() {
            return;
        }
        if self
            .source
            .as_ref()
            .is_some_and(|s| s.matches(&canvas.editor, frame))
        {
            return;
        }
        if self.source.is_some() {
            self.revision = self
                .revision
                .and_then(|v| v.checked_add(1))
                .filter(|v| *v <= MAX_INTEGER);
            self.identity = Arc::new(());
        }
        self.source = Some(Source::capture(canvas, frame));
    }
    pub(super) fn revision(&self) -> Option<u64> {
        self.revision
    }
    pub(super) fn stamp(&self) -> Stamp {
        Stamp {
            store: self.store.stamp(),
            source: self.identity.clone(),
        }
    }
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot {
            store: self.store.clone(),
            revision: self.revision,
        }
    }
}
impl Stamp {
    pub(super) fn matches(&self, owner: &Owner) -> bool {
        self.store.matches(&owner.store) && Arc::ptr_eq(&self.source, &owner.identity)
    }
}
impl Snapshot {
    pub(super) fn has_source_revision(&self) -> bool {
        self.revision.is_some()
    }
    pub(super) fn write(
        &self,
        fields: &mut Map<String, Value>,
    ) -> Result<(), crate::unit_attachment_store::Error> {
        self.store.write(fields)?;
        // 未知的历史 propagation 不用默认对象覆盖；相应来源不能授权翻译视图。
        if let Some(revision) = self.revision {
            let propagation = fields
                .entry("stickerEditPropagation")
                .or_insert_with(|| Value::Object(Map::new()));
            // Option<RawValue> 把 JSON null 当作缺省；保存时按同一语义补成来源对象。
            if propagation.is_null() {
                *propagation = Value::Object(Map::new());
            }
            let propagation = propagation
                .as_object_mut()
                .ok_or(crate::unit_attachment_store::Error::Invalid)?;
            propagation.insert("revision".into(), revision.into());
        }
        Ok(())
    }
}
pub(super) fn observe(state: &State) {
    state.extension_document.borrow_mut().observe(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
    );
}
