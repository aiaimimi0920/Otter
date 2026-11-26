//! Raster edits and dependent blur snapshots form one transaction; background preparation alters neither documents nor history.
use super::*;
use crate::{blur, blur_image::BlurImage, erase_layers::Layers};
use std::{collections::BTreeMap, sync::Arc};

pub struct RasterRefreshPlan {
    identity: Arc<()>,
    revision: u64,
    source: Layers,
    candidate: Layers,
    replacements: Vec<(usize, Stroke)>,
    extra: usize,
    removed: Vec<usize>,
    prepared: bool,
    geometry_change: bool,
    annotation_frame: Option<crate::crop_geometry::Rect>,
    base_projection: Option<crate::image_projection::Projection>,
    base_mapping_changed: bool,
    full_logical_frame: Option<crate::crop_geometry::Rect>,
    crop_change: Option<Option<CropView>>,
    mirror_change: Option<MirrorState>,
}

impl RasterRefreshPlan {
    pub(super) fn with_mirror_target(mut self, next: MirrorState) -> Self {
        self.mirror_change = Some(next);
        self
    }
    pub fn requires_effect_compute(&self) -> bool {
        self.replacements
            .iter()
            .any(|(_, stroke)| stroke.tool == Tool::Blur)
            && (!self.candidate.same_storage(&self.source)
                || self.base_mapping_changed
                || (self.crop_change.is_some() && self.candidate.annotations.is_some()))
    }
    pub(super) fn with_crop_target(mut self, next: Option<CropView>) -> Self {
        let base_projection =
            super::crop::sampling::base_for_view(next, self.full_logical_frame, self.source.size);
        self.base_mapping_changed = self.base_projection != base_projection;
        self.base_projection = base_projection;
        self.annotation_frame = next
            .map(|view| view.source.rect)
            .or(self.full_logical_frame);
        self.crop_change = Some(next);
        self.geometry_change = true;
        self
    }
    pub(super) fn for_transform(
        editor: &Editor,
        replacements: Vec<Stroke>,
    ) -> Result<Self, EditError> {
        let source = editor
            .raster
            .as_ref()
            .ok_or(EditError::InvalidPoint)?
            .clone();
        if replacements.len() != editor.elements.len() {
            return Err(EditError::InvalidPoint);
        }
        let extra = replacements
            .iter()
            .map(|stroke| stroke.points.len())
            .sum::<usize>();
        if editor.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let geometry_change = !replacements.is_empty();
        Ok(Self {
            identity: editor.identity.clone(),
            revision: editor.revision,
            candidate: source.clone(),
            source,
            replacements: replacements.into_iter().enumerate().collect(),
            extra,
            removed: Vec::new(),
            prepared: false,
            geometry_change,
            annotation_frame: editor.annotation_frame(),
            base_projection: editor.raster_base_projection(),
            base_mapping_changed: false,
            full_logical_frame: editor.full_logical_frame(),
            crop_change: None,
            mirror_change: None,
        })
    }
    pub fn matches_source(&self, source: &Layers, revision: u64) -> bool {
        self.revision == revision && self.source.same_storage(source)
    }
    /// Erase threads deliver candidates without changing dimensions or bypassing buffer-length checks.
    pub fn with_candidate(mut self, candidate: Layers) -> Result<Self, blur::BlurError> {
        if candidate.size != self.source.size || !candidate.valid() {
            return Err(blur::BlurError::InvalidInput);
        }
        if !self.removed.is_empty()
            && (!candidate.base.same_storage(&self.source.base) || candidate.annotations.is_none())
        {
            return Err(blur::BlurError::InvalidInput);
        }
        self.candidate = candidate;
        self.prepared = false;
        Ok(self)
    }
    /// Hosts must call in background. Compose once, share equal-intensity results, and never commit failures/cancellation.
    pub fn compute(mut self, cancelled: impl Fn() -> bool) -> Result<Self, blur::BlurError> {
        if cancelled() {
            return Err(blur::BlurError::Cancelled);
        }
        if self.requires_effect_compute() {
            let source = crate::raster_source::compose_in_frames(
                &self.candidate,
                self.base_projection,
                self.annotation_frame,
                &cancelled,
            )?;
            let (width, height) = self.candidate.size;
            // Accepted blur snapshots can have independent sources/frames. Only freshly computed
            // results share by strength; old pixels are not evidence of the current raster.
            let mut cache = BTreeMap::<u32, BlurImage>::new();
            for (_, stroke) in &mut self.replacements {
                if stroke.tool != Tool::Blur {
                    continue;
                }
                let old_blur = stroke.blur.as_ref().ok_or(blur::BlurError::InvalidInput)?;
                let strength = old_blur.strength();
                let key = strength.to_bits();
                let image = if let Some(image) = cache.get(&key) {
                    image.clone()
                } else {
                    let image = BlurImage::prepare(
                        &source,
                        (width, height),
                        blur::Region {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        strength,
                        self.base_projection,
                        &cancelled,
                    )?;
                    cache.insert(key, image.clone());
                    image
                };
                stroke.blur = Some(image);
            }
        }
        if cancelled() {
            return Err(blur::BlurError::Cancelled);
        }
        self.prepared = true;
        Ok(self)
    }
}

impl Editor {
    /// Capture document/blur geometry on UI threads, leaving pixel processing to background workers.
    pub fn prepare_raster_edit(&self) -> Result<Option<RasterRefreshPlan>, EditError> {
        let source = self.raster.as_ref().ok_or(EditError::InvalidPoint)?.clone();
        self.prepare_raster_refresh_inner(source, true, &[])
    }
    pub fn prepare_rasterize_edit(
        &self,
        indices: &[usize],
    ) -> Result<Option<RasterRefreshPlan>, EditError> {
        if indices.is_empty()
            || indices.iter().any(|i| *i >= self.elements.len())
            || !indices.windows(2).all(|p| p[0] < p[1])
        {
            return Err(EditError::InvalidPoint);
        }
        let source = self.raster.as_ref().ok_or(EditError::InvalidPoint)?.clone();
        self.prepare_raster_refresh_inner(source, true, indices)
    }
    pub fn prepare_raster_refresh(
        &self,
        candidate: Layers,
    ) -> Result<Option<RasterRefreshPlan>, EditError> {
        self.prepare_raster_refresh_inner(candidate, false, &[])
    }
    fn prepare_raster_refresh_inner(
        &self,
        candidate: Layers,
        pending: bool,
        removed: &[usize],
    ) -> Result<Option<RasterRefreshPlan>, EditError> {
        let source = self.raster.as_ref().ok_or(EditError::InvalidPoint)?;
        if candidate.size != source.size || !candidate.valid() {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some() || (!pending && candidate.same_storage(source)) {
            return Ok(None);
        }
        let extra = self
            .elements
            .iter()
            .enumerate()
            .filter(|(i, stroke)| stroke.tool == Tool::Blur && removed.binary_search(i).is_err())
            .map(|(_, stroke)| stroke.points.len())
            .sum::<usize>();
        if self.retained_points + extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let replacements = self
            .elements
            .iter()
            .enumerate()
            .filter(|(i, stroke)| stroke.tool == Tool::Blur && removed.binary_search(i).is_err())
            .map(|(index, stroke)| (index, stroke.clone()))
            .collect();
        Ok(Some(RasterRefreshPlan {
            identity: self.identity.clone(),
            revision: self.revision,
            source: source.clone(),
            candidate,
            replacements,
            extra,
            removed: removed.to_vec(),
            prepared: false,
            geometry_change: false,
            annotation_frame: self.annotation_frame(),
            base_projection: self.raster_base_projection(),
            base_mapping_changed: false,
            full_logical_frame: self.full_logical_frame(),
            crop_change: None,
            mirror_change: None,
        }))
    }

    pub fn commit_raster_refresh(
        &mut self,
        mut plan: RasterRefreshPlan,
    ) -> Result<bool, EditError> {
        if !plan.prepared
            || (!plan.geometry_change && plan.candidate.same_storage(&plan.source))
            || !Arc::ptr_eq(&self.identity, &plan.identity)
            || self.revision != plan.revision
            || self.draft.is_some()
            || !self
                .raster
                .as_ref()
                .is_some_and(|source| source.same_storage(&plan.source))
        {
            return Ok(false);
        }
        if self.retained_points + plan.extra > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        for (index, stroke) in &mut plan.replacements {
            std::mem::swap(stroke, &mut self.elements[*index]);
            *index -= plan.removed.partition_point(|removed| removed < index);
        }
        let mut removed = Vec::with_capacity(plan.removed.len());
        for index in plan.removed.into_iter().rev() {
            removed.push((index, Some(self.elements.remove(index))));
        }
        removed.reverse();
        let previous = self.raster.replace(plan.candidate).unwrap();
        self.retained_points += plan.extra;
        let mut entry = Entry::raster_refreshed(previous, plan.replacements, removed);
        if let Some(next) = plan.crop_change {
            entry = entry.with_crop(std::mem::replace(&mut self.crop, next));
        }
        if let Some(next) = plan.mirror_change {
            entry = entry.with_mirror(std::mem::replace(&mut self.mirror, next));
        }
        self.retained_points -= self.history.record(entry);
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "editor_raster_refresh_tests.rs"]
mod tests;
