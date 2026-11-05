//! 栅格编辑与依赖它的模糊快照作为单一事务；后台准备不会修改文档或历史。
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
}

impl RasterRefreshPlan {
    pub fn requires_effect_compute(&self) -> bool {
        self.replacements
            .iter()
            .any(|(_, stroke)| stroke.tool == Tool::Blur)
            && (!self.candidate.same_storage(&self.source)
                || self.base_mapping_changed
                || (self.crop_change.is_some() && self.candidate.annotations.is_some()))
    }
    pub(super) fn with_crop_target(mut self, next: Option<CropView>) -> Self {
        let base_projection = next.and_then(|view| view.base_projection(self.source.size));
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
        })
    }
    pub fn matches_source(&self, source: &Layers, revision: u64) -> bool {
        self.revision == revision && self.source.same_storage(source)
    }
    /// 擦除线程交付候选；不允许更换尺寸或绕过缓冲长度检查。
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
    /// 宿主必须在后台调用。合成一次，相同强度共享结果；失败或取消不可提交。
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
            let mut cache = BTreeMap::<u32, BlurImage>::new();
            for (_, stroke) in &mut self.replacements {
                if stroke.tool != Tool::Blur {
                    continue;
                }
                let strength = stroke
                    .blur
                    .as_ref()
                    .ok_or(blur::BlurError::InvalidInput)?
                    .strength();
                let key = strength.to_bits();
                let image = if let Some(image) = cache.get(&key) {
                    image.clone()
                } else {
                    let pixels = blur::region_precise(
                        &source,
                        (width, height),
                        blur::Region {
                            x: 0,
                            y: 0,
                            width,
                            height,
                        },
                        strength,
                        &cancelled,
                    )?;
                    let image = BlurImage::with_strength(width, height, pixels, strength)
                        .ok_or(blur::BlurError::InvalidInput)?;
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
    /// 在 UI 线程捕获文档及模糊几何，像素处理留给后台。
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
        self.retained_points -= self.history.record(entry);
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (16, 16),
            base: [0, 0, 255, 255].repeat(256).into(),
            annotations: None,
        })
        .unwrap();
        for strength in [2.4, 2.4, 4.0] {
            let image =
                BlurImage::with_strength(16, 16, [0, 0, 255, 255].repeat(256).into(), strength)
                    .unwrap();
            editor.begin_blur(Point { x: 8.0, y: 8.0 }, image).unwrap();
            assert!(editor.commit());
        }
        editor
    }
    fn candidate(editor: &Editor) -> Layers {
        let mut layers = editor.layers().unwrap().clone();
        layers.annotations = Some([255, 0, 0, 255].repeat(256).into());
        layers
    }
    #[test]
    fn raster_and_all_blurs_commit_and_undo_together_with_shared_strength_results() {
        let mut editor = document();
        let original = editor.layers().unwrap().clone();
        let old = editor.elements()[0].blur.clone().unwrap();
        let points = editor.elements()[0].points.as_ptr();
        let revision = editor.revision();
        let next = candidate(&editor);
        let plan = editor
            .prepare_raster_refresh(next.clone())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert_eq!(editor.revision(), revision);
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        let blur = editor.elements()[0].blur.as_ref().unwrap();
        assert_eq!(blur.strength(), 2.4);
        assert!(blur
            .pixels()
            .same_storage(editor.elements()[1].blur.as_ref().unwrap().pixels()));
        let expected = blur::region_precise(
            next.annotations.as_ref().unwrap(),
            next.size,
            blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            2.4,
            || false,
        )
        .unwrap();
        assert_eq!(&blur.pixels()[..], &expected[..]);
        for _ in 0..3 {
            assert!(editor.undo());
            assert!(editor.layers().unwrap().same_storage(&original));
            assert!(editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(old.pixels()));
            assert_eq!(editor.elements()[0].points.as_ptr(), points);
            assert!(editor.redo());
            assert!(editor.layers().unwrap().same_storage(&next));
        }
    }
    #[test]
    fn rasterizing_blurs_excludes_removed_effects_and_balances_point_budget() {
        for indices in [vec![0], vec![1], vec![0, 2], vec![0, 1, 2]] {
            let mut editor = document();
            let retained = editor.retained_points;
            let layers = candidate(&editor);
            let plan = editor.prepare_rasterize_edit(&indices).unwrap().unwrap();
            assert_eq!(plan.replacements.len(), 3 - indices.len());
            let extra = plan.extra;
            let plan = plan
                .with_candidate(layers)
                .unwrap()
                .compute(|| false)
                .unwrap();
            assert!(editor.commit_raster_refresh(plan).unwrap());
            assert_eq!(editor.elements().len(), 3 - indices.len());
            assert_eq!(editor.retained_points, retained + extra);
            assert!(editor.undo());
            assert_eq!(editor.elements().len(), 3);
            editor.begin(Tool::Brush, Point::default()).unwrap();
            assert!(editor.commit());
            assert_eq!(editor.retained_points, retained + 1);
        }
    }

    #[test]
    fn pending_edit_with_unchanged_layers_is_noop_and_preserves_redo() {
        let mut editor = document();
        assert!(editor.undo());
        let revision = editor.revision();
        let plan = editor.prepare_raster_edit().unwrap().unwrap();
        assert!(plan.matches_source(editor.layers().unwrap(), revision));
        assert!(!plan.matches_source(editor.layers().unwrap(), revision + 1));
        let plan = plan.compute(|| false).unwrap();
        assert!(!editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
    }

    #[test]
    fn cancellation_unprepared_foreign_and_stale_candidates_never_change_document() {
        let mut editor = document();
        let original = editor.layers().unwrap().clone();
        let plan = editor
            .prepare_raster_refresh(candidate(&editor))
            .unwrap()
            .unwrap();
        assert!(!editor.commit_raster_refresh(plan).unwrap());
        let plan = editor
            .prepare_raster_refresh(candidate(&editor))
            .unwrap()
            .unwrap();
        assert!(matches!(
            plan.compute(|| true),
            Err(blur::BlurError::Cancelled)
        ));
        let plan = editor
            .prepare_raster_refresh(candidate(&editor))
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        let mut other = document();
        assert!(!other.commit_raster_refresh(plan).unwrap());
        let plan = editor
            .prepare_raster_refresh(candidate(&editor))
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        editor.undo();
        assert!(!editor.commit_raster_refresh(plan).unwrap());
        assert!(editor.layers().unwrap().same_storage(&original));
        assert!(editor.redo());
    }
}
