//! 栅格化原子事务：图层替换和矢量移除共享一个历史项，保留原索引与对象所有权。
use super::*;
use crate::erase_layers::Layers;

impl Editor {
    /// 只克隆待栅格化矢量，不携带底图、草稿或历史；用于独立后台合成。
    pub fn rasterize_snapshot(&self, indices: &[usize]) -> Result<Self, EditError> {
        if indices.is_empty()
            || indices.iter().any(|i| *i >= self.elements.len())
            || !indices.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(EditError::InvalidPoint);
        }
        let elements: Vec<_> = indices.iter().map(|i| self.elements[*i].clone()).collect();
        let retained_points = elements.iter().map(|stroke| stroke.points.len()).sum();
        Ok(Self {
            elements,
            retained_points,
            revision: self.revision,
            style: self.style,
            crop: self.crop,
            ..Self::default()
        })
    }
    /// indices 必须严格递增；后台合成完成后调用，过期结果不修改文档。
    pub fn commit_rasterized(
        &mut self,
        revision: u64,
        source: &Layers,
        candidate: Layers,
        indices: &[usize],
    ) -> Result<bool, EditError> {
        let valid_indices = !indices.is_empty()
            && indices.len() <= self.elements.len()
            && indices.iter().all(|index| *index < self.elements.len())
            && indices.windows(2).all(|pair| pair[0] < pair[1]);
        let valid_pixels = candidate.size == source.size
            && candidate.base.same_storage(&source.base)
            && candidate.valid()
            && candidate.annotations.is_some();
        if !valid_indices || !valid_pixels {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some()
            || self.revision != revision
            || !self
                .raster
                .as_ref()
                .is_some_and(|layers| layers.same_storage(source))
        {
            return Ok(false);
        }
        let mut removed = Vec::with_capacity(indices.len());
        for index in indices.iter().rev() {
            removed.push((*index, Some(self.elements.remove(*index))));
        }
        removed.reverse();
        let previous = self.raster.replace(candidate).unwrap();
        self.retained_points -= self.history.record(Entry::rasterized(previous, removed));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> Editor {
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![255; 16 * 16 * 4].into(),
            annotations: None,
        })
        .unwrap();
        for x in [2.0, 5.0, 8.0, 11.0] {
            editor.begin(Tool::Brush, Point { x, y: 8.0 }).unwrap();
            assert!(editor.commit());
        }
        editor
    }
    fn candidate(source: &Layers) -> Layers {
        Layers {
            annotation_size: None,
            annotations: Some(vec![128; 16 * 16 * 4].into()),
            ..source.clone()
        }
    }
    #[test]
    fn selected_rasterization_restores_exact_vector_order_and_layer_identity() {
        let mut editor = editor();
        let source = editor.layers().unwrap().clone();
        let candidate = candidate(&source);
        let retained = editor.retained_points;
        let allocations: Vec<_> = editor
            .elements()
            .iter()
            .map(|stroke| stroke.points.as_ptr())
            .collect();
        assert!(editor
            .commit_rasterized(editor.revision(), &source, candidate.clone(), &[1, 3])
            .unwrap());
        assert_eq!(
            editor
                .elements()
                .iter()
                .map(|s| s.points[0].x)
                .collect::<Vec<_>>(),
            vec![2.0, 8.0]
        );
        assert_eq!(editor.retained_points, retained);
        for _ in 0..3 {
            assert!(editor.undo());
            assert!(editor.layers().unwrap().same_storage(&source));
            assert_eq!(
                editor
                    .elements()
                    .iter()
                    .map(|s| s.points.as_ptr())
                    .collect::<Vec<_>>(),
                allocations
            );
            assert!(editor.redo());
            assert!(editor.layers().unwrap().same_storage(&candidate));
            assert_eq!(editor.elements().len(), 2);
        }
    }
    #[test]
    fn invalid_and_stale_candidates_cannot_remove_any_vector() {
        let mut editor = editor();
        let source = editor.layers().unwrap().clone();
        let revision = editor.revision();
        for indices in [vec![], vec![1, 1], vec![3, 1], vec![4]] {
            assert!(editor
                .commit_rasterized(revision, &source, candidate(&source), &indices)
                .is_err());
        }
        assert!(!editor
            .commit_rasterized(revision + 1, &source, candidate(&source), &[0])
            .unwrap());
        let mut invalid = candidate(&source);
        invalid.base = vec![0; 16 * 16 * 4].into();
        assert!(editor
            .commit_rasterized(revision, &source, invalid, &[0])
            .is_err());
        assert_eq!(editor.elements().len(), 4);
        assert_eq!(editor.revision(), revision);
        assert!(editor.layers().unwrap().same_storage(&source));
    }
    #[test]
    fn all_rasterization_can_be_undone_before_branch_edit_without_losing_vectors() {
        let mut editor = editor();
        let source = editor.layers().unwrap().clone();
        let output = candidate(&source);
        let weak = output.annotations.as_ref().unwrap().downgrade();
        assert!(editor
            .commit_rasterized(editor.revision(), &source, output, &[0, 1, 2, 3])
            .unwrap());
        assert!(editor.elements().is_empty());
        assert!(editor.undo());
        editor
            .begin(Tool::Brush, Point { x: 14.0, y: 8.0 })
            .unwrap();
        assert!(editor.commit());
        assert!(!editor.redo());
        assert!(weak.upgrade().is_none());
        assert_eq!(editor.elements().len(), 5);
        assert_eq!(editor.retained_points, 5);
    }
}
