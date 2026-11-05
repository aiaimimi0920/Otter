//! 外部源推进不是本地 undo；仅在所有历史底图仍共享同一基线时原子重基。
use super::*;
use crate::erase_layers::Layers;
fn compatible(layers: &Layers, source: &Layers) -> bool {
    layers.size == source.size && layers.base.same_storage(&source.base)
}
fn safe(stroke: &Stroke) -> bool {
    stroke.blur.is_none()
}
impl History {
    pub(crate) fn source_compatible(&self, source: &Layers) -> bool {
        self.entries.iter().all(|entry| {
            entry.document.as_ref().is_none_or(|doc| {
                doc.raster
                    .as_ref()
                    .is_some_and(|layers| compatible(layers, source))
                    && doc.elements.iter().all(safe)
            }) && entry
                .raster
                .as_ref()
                .is_none_or(|layers| compatible(layers, source))
                && entry.stored.as_ref().is_none_or(safe)
                && entry
                    .batch
                    .as_ref()
                    .is_none_or(|items| items.iter().all(|(_, s)| s.as_ref().is_none_or(safe)))
                && entry
                    .replacements
                    .as_ref()
                    .is_none_or(|items| items.iter().all(|(_, s)| safe(s)))
        })
    }
    /// 调用前必须完成 source_compatible；只换共享底图，不动 cursor/标注/裁剪。
    pub(crate) fn rebase_source(&mut self, base: &crate::pixels::Pixels) {
        for entry in &mut self.entries {
            if let Some(layers) = entry.document.as_mut().and_then(|doc| doc.raster.as_mut()) {
                layers.base = base.clone();
            }
            if let Some(layers) = &mut entry.raster {
                layers.base = base.clone();
            }
        }
    }
}
