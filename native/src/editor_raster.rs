//! 栅格事务加入同一历史游标；异步提交必须同时匹配版本与原始快照身份。
use super::*;
use crate::erase_layers::Layers;
#[path = "editor_source.rs"]
mod source;

fn valid(layers: &Layers) -> bool {
    layers.valid()
}

impl Editor {
    /// 仅创建新文档时设置底图，不能用此接口绕过已有历史替换图片。
    pub fn with_layers(layers: Layers) -> Result<Self, EditError> {
        if !valid(&layers) {
            return Err(EditError::InvalidPoint);
        }
        Ok(Self {
            raster: Some(layers),
            ..Self::default()
        })
    }

    pub fn layers(&self) -> Option<&Layers> {
        self.raster.as_ref()
    }

    pub fn replace_layers(
        &mut self,
        expected_revision: u64,
        source: &Layers,
        candidate: Layers,
    ) -> Result<bool, EditError> {
        if !valid(&candidate) || candidate.size != source.size {
            return Err(EditError::InvalidPoint);
        }
        if self.draft.is_some()
            || self.revision != expected_revision
            || !self
                .raster
                .as_ref()
                .is_some_and(|current| current.same_storage(source))
        {
            return Ok(false);
        }
        if candidate.same_storage(source) {
            return Ok(false);
        }
        let previous = self.raster.replace(candidate).unwrap();
        self.retained_points -= self.history.record(Entry::raster(previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layers(value: u8) -> Layers {
        Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![value; 16 * 16 * 4].into(),
            annotations: None,
        }
    }
    fn stroke(editor: &mut Editor, x: f32) {
        editor.begin(Tool::Brush, Point { x, y: 8.0 }).unwrap();
        assert!(editor.commit());
    }
    #[test]
    fn raster_and_vector_undo_in_one_order_without_copying_pixels() {
        let original = layers(255);
        let erased = original
            .erase(
                crate::erase_layers::Mode::Content,
                &[Point { x: 8.0, y: 8.0 }],
                8.0,
                || false,
            )
            .unwrap();
        let mut editor = Editor::with_layers(original.clone()).unwrap();
        stroke(&mut editor, 2.0);
        assert!(editor
            .replace_layers(editor.revision(), &original, erased.clone())
            .unwrap());
        stroke(&mut editor, 12.0);
        assert!(editor.undo());
        assert_eq!(editor.elements().len(), 1);
        assert!(editor.layers().unwrap().same_storage(&erased));
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&original));
        assert!(editor.undo());
        assert!(editor.elements().is_empty());
        for _ in 0..3 {
            assert!(editor.redo());
        }
        assert_eq!(editor.elements().len(), 2);
        assert!(editor.layers().unwrap().same_storage(&erased));
        assert!(editor
            .export_snapshot()
            .layers()
            .unwrap()
            .same_storage(&erased));
    }
    #[test]
    fn rejected_results_and_noops_preserve_history_and_revision() {
        let original = layers(255);
        let mut editor = Editor::with_layers(original.clone()).unwrap();
        assert!(!editor.replace_layers(1, &original, layers(0)).unwrap());
        assert!(!editor.replace_layers(0, &layers(255), layers(0)).unwrap());
        assert!(!editor
            .replace_layers(0, &original, original.clone())
            .unwrap());
        editor.begin(Tool::Brush, Point { x: 2.0, y: 2.0 }).unwrap();
        assert!(!editor.replace_layers(0, &original, layers(0)).unwrap());
        editor.cancel();
        assert_eq!(editor.revision(), 0);
        assert!(!editor.undo());
        assert!(editor.layers().unwrap().same_storage(&original));
    }
    #[test]
    fn branching_releases_discarded_raster_and_old_history_is_bounded() {
        let original = layers(255);
        let weak = original.base.downgrade();
        let mut editor = Editor::with_layers(original).unwrap();
        for _ in 0..51 {
            let source = editor.layers().unwrap().clone();
            assert!(editor
                .replace_layers(editor.revision(), &source, layers(128))
                .unwrap());
        }
        assert!(weak.upgrade().is_none());
        let current = editor.layers().unwrap().base.downgrade();
        assert!(editor.undo());
        stroke(&mut editor, 2.0);
        assert!(!editor.redo());
        assert!(current.upgrade().is_none());
    }
}
