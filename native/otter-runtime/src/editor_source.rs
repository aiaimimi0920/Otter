//! Nonundoable remote-source baseline advancement; preserve local layers, geometry, and every undo/redo cursor.
use super::*;
impl Editor {
    pub fn source_replaceable(&self) -> bool {
        self.draft.is_none()
            && self.elements.iter().all(|stroke| stroke.blur.is_none())
            && self
                .raster
                .as_ref()
                .is_some_and(|layers| self.history.source_compatible(layers))
    }
    pub fn replace_source(&mut self, base: crate::pixels::Pixels) -> bool {
        if !self.source_replaceable()
            || self
                .raster
                .as_ref()
                .is_none_or(|layers| layers.base.len() != base.len())
        {
            return false;
        }
        self.history.rebase_source(&base);
        self.raster.as_mut().unwrap().base = base;
        self.revision = self.revision.wrapping_add(1);
        true
    }
}
#[cfg(test)]
#[path = "editor_source_tests.rs"]
mod tests;
