//! Committed-content snapshots share pixels and isolate vectors, history, drafts, and document identity.
use super::*;

impl Editor {
    /// Bind a display-only snapshot to its pixels without replacing existing raster edits.
    pub fn duplicate_snapshot_with_fallback_base(
        &self,
        base: crate::erase_layers::Layers,
    ) -> Result<Self, EditError> {
        if self.raster.is_none() && !base.valid() {
            return Err(EditError::InvalidPoint);
        }
        let mut snapshot = self.duplicate_snapshot()?;
        if snapshot.raster.is_none() {
            snapshot.raster = Some(base);
        }
        Ok(snapshot)
    }
    /// A pasted document owns fresh annotation identities and no source undo history.
    pub fn duplicate_snapshot(&self) -> Result<Self, EditError> {
        let mut snapshot = self.export_snapshot();
        for stroke in &mut snapshot.elements {
            stroke.annotation_id = annotation_id::AnnotationId::new()?;
        }
        Ok(snapshot)
    }
    /// Export candidates replace effect resources independently; discard local snapshots on failure without active-history/revision writes.
    pub fn export_snapshot_with_blurs<E>(
        &self,
        mut prepare: impl FnMut(
            &crate::blur_image::BlurImage,
        ) -> Result<crate::blur_image::BlurImage, E>,
    ) -> Result<Self, E> {
        self.export_snapshot_with_blur_strokes(|stroke| prepare(stroke.blur.as_ref().unwrap()))
    }
    /// Prepare local effects by stable annotation identity; closures read original strokes only, replacing within independent snapshots.
    pub fn export_snapshot_with_blur_strokes<E>(
        &self,
        mut prepare: impl FnMut(&Stroke) -> Result<crate::blur_image::BlurImage, E>,
    ) -> Result<Self, E> {
        let mut snapshot = self.export_snapshot();
        for stroke in &mut snapshot.elements {
            if stroke.blur.is_some() {
                stroke.blur = Some(prepare(stroke)?);
            }
        }
        Ok(snapshot)
    }
    /// Bind current displayed bases while temporarily editing Art overlays; preserve original documents and exclude old raster contents.
    pub fn export_overlay_with_base(
        &self,
        base: crate::erase_layers::Layers,
        measure: impl FnMut(&crate::text_annotation::TextAnnotation) -> Result<Point, EditError>,
    ) -> Result<Self, EditError> {
        if !base.valid() {
            return Err(EditError::InvalidPoint);
        }
        let mut snapshot = self.export_overlay_for_size(base.size, measure)?;
        snapshot.raster = Some(base);
        Ok(snapshot)
    }
    /// Art display uses only vector/crop overlays; generated results supply bases, excluding old raster history.
    pub fn export_overlay_snapshot(&self) -> Self {
        let mut snapshot = self.export_snapshot();
        snapshot.raster = None;
        snapshot.mirror = MirrorState::default();
        snapshot
    }
    /// Collaboration basis removes shareable vectors only; preserve materialized rasters, crops, and logical geometry unchanged.
    pub fn export_without_annotations(&self) -> Self {
        self.snapshot_with_elements(Vec::new())
    }
    /// Export copies committed annotations only, without drafts, redo branches, or image pixels.
    pub fn export_snapshot(&self) -> Self {
        self.snapshot_with_elements(self.elements().to_vec())
    }
    // Annotation-free basis must not copy all strokes just to discard them; both snapshot types share other model state.
    fn snapshot_with_elements(&self, elements: Vec<Stroke>) -> Self {
        let retained_points = elements.iter().map(|stroke| stroke.points.len()).sum();
        Self {
            style: self.style,
            identity: std::sync::Arc::default(),
            revision: self.revision,
            presentation_revision: self.presentation_revision,
            serial_count: self.serial_count,
            spotlight_percent: self.spotlight_percent,
            history: History::default(),
            elements,
            retained_points,
            draft: None,
            draft_motion: None,
            draft_brush_highlighter: false,
            raster: self.raster.clone(),
            crop: self.crop,
            logical_size: self.logical_size,
            mirror: self.mirror,
            pending_pin: None,
        }
    }
}
