//! 模糊笔迹只接受已计算的不可变图像；计算任务不在编辑事务内运行。
use super::*;
impl Editor {
    pub fn replace_blur(
        &mut self,
        index: usize,
        image: crate::blur_image::BlurImage,
    ) -> Result<bool, EditError> {
        if self.draft.is_some() {
            return Ok(false);
        }
        let Some(source) = self.elements.get(index).filter(|s| s.tool == Tool::Blur) else {
            return Ok(false);
        };
        let Some(previous) = &source.blur else {
            return Ok(false);
        };
        if previous.width() != image.width() || previous.height() != image.height() {
            return Err(EditError::InvalidPoint);
        }
        if previous.strength() == image.strength() && previous.pixels().same_storage(image.pixels())
        {
            return Ok(false);
        }
        if self.retained_points + source.points.len() > MAX_TOTAL_POINTS {
            return Err(EditError::BudgetExceeded);
        }
        let mut changed = source.clone();
        changed.blur = Some(image);
        let old = std::mem::replace(&mut self.elements[index], changed);
        self.retained_points += old.points.len();
        self.retained_points -= self.history.record(Entry::replaced(index, old));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
    pub fn begin_blur(
        &mut self,
        point: Point,
        image: crate::blur_image::BlurImage,
    ) -> Result<bool, EditError> {
        if !self.begin(Tool::Brush, point)? {
            return Ok(false);
        }
        let draft = self.draft.as_mut().unwrap();
        draft.tool = Tool::Blur;
        draft.blur = Some(image);
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_budget_and_dimensions_fail_without_history_or_geometry_changes() {
        let image = crate::blur_image::BlurImage::new(8, 8, vec![0; 8 * 8 * 4].into(), 8).unwrap();
        let mut editor = Editor::default();
        editor.begin_blur(Point::default(), image.clone()).unwrap();
        editor.commit();
        let revision = editor.revision();
        assert!(!editor.replace_blur(0, image).unwrap());
        let wrong = crate::blur_image::BlurImage::new(4, 4, vec![0; 4 * 4 * 4].into(), 10).unwrap();
        assert_eq!(editor.replace_blur(0, wrong), Err(EditError::InvalidPoint));
        editor.retained_points = MAX_TOTAL_POINTS;
        let next = crate::blur_image::BlurImage::new(8, 8, vec![0; 8 * 8 * 4].into(), 10).unwrap();
        assert_eq!(editor.replace_blur(0, next), Err(EditError::BudgetExceeded));
        assert_eq!(editor.revision(), revision);
        assert_eq!(editor.elements()[0].blur.as_ref().unwrap().sigma(), 8);
    }
    #[test]
    fn blur_draft_history_and_snapshot_share_pixels_and_cancel_releases_them() {
        let pixels: crate::pixels::Pixels = vec![0; 16 * 16 * 4].into();
        let image = crate::blur_image::BlurImage::new(16, 16, pixels.clone(), 8).unwrap();
        let mut editor = Editor::default();
        assert!(!editor.begin(Tool::Blur, Point::default()).unwrap());
        assert!(editor
            .begin_blur(Point { x: 2.0, y: 2.0 }, image.clone())
            .unwrap());
        editor.extend(Point { x: 5.0, y: 8.0 }).unwrap();
        editor.extend(Point { x: 10.0, y: 12.0 }).unwrap();
        assert!(editor.commit());
        assert_eq!(editor.elements()[0].points.len(), 3);
        let snapshot = editor.export_snapshot();
        assert!(snapshot.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&pixels));
        editor.undo();
        editor.redo();
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&pixels));
        editor.begin_blur(Point::default(), image).unwrap();
        assert!(editor.cancel());
        assert_eq!(editor.elements().len(), 1);
    }
}
