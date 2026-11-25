//! External-source advancement is not local undo; atomically rebase only while every historical base still shares the same baseline.
use super::*;
use crate::erase_layers::Layers;
fn compatible(layers: &Layers, source: &Layers) -> bool {
    layers.size == source.size && layers.base.same_storage(&source.base)
}
fn safe(stroke: &Stroke) -> bool {
    stroke.blur.is_none()
}
impl History {
    pub fn source_compatible(&self, source: &Layers) -> bool {
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
                && entry
                    .annotations
                    .as_ref()
                    .is_none_or(|annotations| annotations.elements.iter().all(safe))
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
    /// Complete source_compatible before calling; replace shared bases only, leaving cursor/annotations/crops unchanged.
    pub fn rebase_source(&mut self, base: &crate::pixels::Pixels) {
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
