//! Atomic rasterization: layer replacement/vector removal share one history item, preserving original indices/object ownership.
use super::*;
use crate::erase_layers::Layers;

impl Editor {
    /// The single raster annotation layer sits below all vector effects. Never move
    /// upper vectors below a retained spotlight or into a magnifier's sampling surface.
    pub fn rasterizable_indices(&self) -> impl Iterator<Item = usize> + '_ {
        let dynamic = self
            .elements
            .iter()
            .chain(self.draft.iter())
            .any(|stroke| stroke.tool.spotlight() || stroke.tool == Tool::Magnifier);
        self.elements
            .iter()
            .enumerate()
            .filter_map(move |(index, stroke)| {
                let rank = stroke.tool.render_rank();
                (rank < 3 || (!dynamic && rank == 5)).then_some(index)
            })
    }
    pub(super) fn validate_rasterize_indices(&self, indices: &[usize]) -> Result<(), EditError> {
        let mut allowed = self.rasterizable_indices();
        // Consume one ordered iterator: this also rejects duplicates and descending indices.
        if indices.is_empty()
            || !indices
                .iter()
                .all(|index| allowed.by_ref().any(|candidate| candidate == *index))
        {
            return Err(EditError::InvalidPoint);
        }
        Ok(())
    }
    /// Clone only vectors pending rasterization for independent background composition, without bases, drafts, or history.
    pub fn rasterize_snapshot(&self, indices: &[usize]) -> Result<Self, EditError> {
        self.validate_rasterize_indices(indices)?;
        let elements: Vec<_> = indices.iter().map(|i| self.elements[*i].clone()).collect();
        let retained_points = elements.iter().map(|stroke| stroke.points.len()).sum();
        Ok(Self {
            elements,
            retained_points,
            revision: self.revision,
            style: self.style,
            spotlight_percent: self.spotlight_percent,
            crop: self.crop,
            logical_size: self.logical_size,
            ..Self::default()
        })
    }
    /// indices must strictly increase; call after background composition, with stale results leaving documents unchanged.
    pub fn commit_rasterized(
        &mut self,
        revision: u64,
        source: &Layers,
        candidate: Layers,
        indices: &[usize],
    ) -> Result<bool, EditError> {
        self.validate_rasterize_indices(indices)?;
        let valid_pixels = candidate.size == source.size
            && candidate.base.same_storage(&source.base)
            && candidate.valid()
            && candidate.annotations.is_some();
        if !valid_pixels {
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

    #[test]
    fn dynamic_composition_policy_is_shared_by_every_rasterization_entry() {
        for tool in [
            Tool::SpotRect,
            Tool::SpotRounded,
            Tool::SpotEllipse,
            Tool::Magnifier,
        ] {
            let mut editor = editor();
            editor
                .begin(Tool::Highlighter, Point { x: 1., y: 1. })
                .unwrap();
            assert!(editor.commit());
            editor.begin(tool, Point { x: 2., y: 2. }).unwrap();
            editor.extend(Point { x: 10., y: 10. }).unwrap();
            assert!(editor.commit());
            assert_eq!(editor.rasterizable_indices().collect::<Vec<_>>(), [4]);
            let source = editor.layers().unwrap().clone();
            let revision = editor.revision();
            for indices in [&[0][..], &[5], &[4, 4], &[4, 0], &[6]] {
                assert!(editor.rasterize_snapshot(indices).is_err());
                assert!(editor.prepare_rasterize_edit(indices).is_err());
                assert!(editor
                    .commit_rasterized(revision, &source, candidate(&source), indices)
                    .is_err());
            }
            assert!(editor.rasterize_snapshot(&[4]).is_ok());
            assert!(editor.prepare_rasterize_edit(&[4]).unwrap().is_some());
            assert_eq!(editor.revision(), revision);
            assert!(editor.layers().unwrap().same_storage(&source));
        }
    }
}
