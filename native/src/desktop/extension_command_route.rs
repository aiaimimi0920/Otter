//! 固定 OCR/翻译入口的主干命令与 when 上下文；只投影元数据，不读取图片/opaque payload。
use super::State;
use crate::{extension_when, loom_extension_snapshot::Snapshot};
pub(super) const OCR: &str = "neuro.official/ocr.recognize-selected-unit";
pub(super) const TRANSLATE: &str = "neuro.official/text-translation.translate";
pub(super) const OCR_OVERLAY: &str = "neuro.official/ocr.toggle-overlay";
pub(super) const TRANSLATION_OVERLAY: &str = "neuro.official/text-translation.toggle-overlay";
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Context {
    art: bool,
    image: bool,
    types: Vec<String>,
}
impl Context {
    pub(super) fn capture(state: &State) -> Self {
        Self {
            art: state.is_art(),
            image: state.resolved_display.borrow().is_some()
                || state.canvas.borrow().image.is_some(),
            types: state
                .extension_document
                .borrow()
                .store
                .items()
                .iter()
                .map(|i| i.attachment.type_id().into())
                .collect(),
        }
    }
    pub(super) fn with<T>(&self, f: impl FnOnce(&extension_when::Context<'_>) -> T) -> T {
        let types: Vec<_> = self.types.iter().map(String::as_str).collect();
        f(&extension_when::Context {
            kind: Some(if self.art {
                extension_when::UnitKind::Art
            } else {
                extension_when::UnitKind::Sticker
            }),
            has_image: self.image,
            attachment_types: &types,
        })
    }
}
pub(super) fn command(catalog: &Snapshot, context: &Context, translation: bool) -> Option<String> {
    let candidates: &[&str] = if translation {
        &[
            TRANSLATION_OVERLAY,
            "neuro.official/text-translation.toggle",
            TRANSLATE,
        ]
    } else {
        &[OCR_OVERLAY, OCR]
    };
    context.with(|context| {
        candidates
            .iter()
            .find(|id| {
                catalog
                    .command(id, context)
                    .is_some_and(|c| c.permissions().is_some())
            })
            .map(|s| (*s).into())
    })
}
