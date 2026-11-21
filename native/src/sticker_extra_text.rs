//! 设置面板的缓存全文读取；不依赖覆盖层显隐或在线目录，不执行扩展命令。
use crate::{extension_visual as visual, unit_attachment_store::Store};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Ocr,
    Translation,
}
#[derive(Clone, Default)]
pub(crate) struct Legacy {
    ocr: Option<Arc<str>>,
    translation: Option<Arc<str>>,
    source_revision: Option<u64>,
}
impl Legacy {
    // 仅后台恢复边界解析旧会话；保留原文本，不改写持久化数据或注册旧扩展。
    pub(crate) fn restore(ocr: Option<&RawValue>, translation: Option<&RawValue>) -> Self {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Cached {
            full_text: Option<String>,
            source_revision: Option<u64>,
        }
        let parse = |raw: Option<&RawValue>| {
            raw.filter(|r| r.get().len() <= 20 * 1024 * 1024)
                .and_then(|r| serde_json::from_str::<Cached>(r.get()).ok())
        };
        let translated = parse(translation);
        Self {
            ocr: parse(ocr).and_then(|v| v.full_text).map(Arc::from),
            translation: translated
                .as_ref()
                .and_then(|v| v.full_text.as_deref())
                .map(Arc::from),
            source_revision: translated.and_then(|v| v.source_revision),
        }
    }
}
pub(crate) fn text<'a>(
    kind: Kind,
    store: &'a Store,
    revision: Option<u64>,
    legacy: &'a Legacy,
) -> Option<&'a str> {
    let (plugin, type_id, key) = match kind {
        Kind::Ocr => (visual::OCR, visual::OCR_RESULT, "fullText"),
        Kind::Translation => (visual::TRANSLATE, visual::TRANSLATION, "text"),
    };
    let text = if let Some(item) = store.items().iter().find(|s| {
        s.attachment.plugin() == plugin
            && s.attachment.type_id() == type_id
            && s.attachment.schema() == "1"
    }) {
        if kind == Kind::Translation
            && !revision.is_some_and(|r| visual::source_current(store, item, r))
        {
            return None;
        }
        item.attachment.payload()?.get(key)?.as_str()
    } else {
        match kind {
            Kind::Ocr => legacy.ocr.as_deref(),
            Kind::Translation if revision.is_some() && revision == legacy.source_revision => {
                legacy.translation.as_deref()
            }
            _ => None,
        }
    };
    text.filter(|s| !s.trim().is_empty())
}
#[cfg(test)]
#[path = "sticker_extra_text_tests.rs"]
pub(crate) mod tests;
