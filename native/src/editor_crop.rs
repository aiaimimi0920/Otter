//! 裁剪只改变文档视图，不移动源坐标中的标注或像素；历史保存固定大小的视图。
use super::*;
use crate::crop_geometry::{self, Crop, Rect};
#[path = "editor_crop_sampling.rs"]
mod sampling;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropView {
    pub frame: Rect,
    pub source: Crop,
    // 旧合同在自然像素中取区域并非等比伸展；不能冒充现代contain源框。
    pub(crate) legacy_pixels: Option<Rect>,
    pub(crate) legacy_natural_size: Option<crate::crop_geometry::Size>,
}

impl Editor {
    pub fn crop_view(&self) -> Option<CropView> {
        self.crop
    }

    /// 栅格标注、效果与输出共用的源坐标画框；普通像素文档保留 None 的默认映射。
    pub fn annotation_frame(&self) -> Option<Rect> {
        self.crop
            .map(|view| view.source.rect)
            .or_else(|| self.full_logical_frame())
    }

    pub(crate) fn full_logical_frame(&self) -> Option<Rect> {
        self.logical_size.map(|size| Rect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        })
    }

    /// 无裁剪时以图像像素对应初始逻辑源框；调用方不能用此接口重采样底图。
    pub fn visible_frame(&self) -> Option<Rect> {
        self.crop.map(|view| view.frame).or_else(|| {
            if let Some(size) = self.logical_size {
                return Some(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: size.width,
                    height: size.height,
                });
            }
            self.raster.as_ref().map(|layers| Rect {
                x: 0.0,
                y: 0.0,
                width: layers.size.0 as f64,
                height: layers.size.1 as f64,
            })
        })
    }

    /// 选区是当前可见框内的局部坐标。越界选区拒绝，而不是隐式扩展文档。
    pub fn crop_to(&mut self, selection: Rect) -> Result<bool, EditError> {
        let frame = self.visible_frame().ok_or(EditError::InvalidPoint)?;
        if selection.x < 0.0
            || selection.y < 0.0
            || selection.x + selection.width > frame.width
            || selection.y + selection.height > frame.height
        {
            return Err(EditError::InvalidPoint);
        }
        let change = crop_geometry::next(frame, self.crop.map(|view| view.source), selection)
            .map_err(|_| EditError::InvalidPoint)?;
        if self.draft.is_some()
            || (selection.x == 0.0
                && selection.y == 0.0
                && selection.width == frame.width
                && selection.height == frame.height)
        {
            return Ok(false);
        }
        let previous = self.crop.replace(CropView {
            frame: change.frame,
            source: change.crop.unwrap(),
            legacy_pixels: self.crop.and_then(|view| view.legacy_pixels),
            legacy_natural_size: self.crop.and_then(|view| view.legacy_natural_size),
        });
        self.retained_points -= self.history.record(Entry::cropped(previous));
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }

    pub fn reset_crop(&mut self) -> bool {
        if self.draft.is_some() || self.crop.is_none() {
            return false;
        }
        let previous = self.crop.take();
        self.retained_points -= self.history.record(Entry::cropped(previous));
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    fn document() -> Editor {
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            annotation_size: None,
            size: (64, 32),
            base: vec![255; 64 * 32 * 4].into(),
            annotations: None,
        })
        .unwrap();
        editor.begin(Tool::Brush, Point { x: 2.0, y: 2.0 }).unwrap();
        assert!(editor.commit());
        editor
    }
    #[test]
    fn repeated_crop_reset_and_history_preserve_outside_geometry_and_pixel_identity() {
        let mut editor = document();
        let original = editor.layers().unwrap().clone();
        let points = editor.elements()[0].points.as_ptr();
        let retained = editor.retained_points;
        assert!(editor.crop_to(rect(10.0, 5.0, 40.0, 20.0)).unwrap());
        let first = editor.crop_view();
        assert!(editor.crop_to(rect(5.0, 4.0, 20.0, 10.0)).unwrap());
        let second = editor.crop_view();
        assert_eq!(second.unwrap().source.rect, rect(15.0, 9.0, 20.0, 10.0));
        let snapshot = editor.export_snapshot();
        assert_eq!(snapshot.crop_view(), second);
        assert!(editor.reset_crop());
        assert_eq!(editor.visible_frame(), Some(rect(0.0, 0.0, 64.0, 32.0)));
        for expected in [second, first, None] {
            assert!(editor.undo());
            assert_eq!(editor.crop_view(), expected);
            assert_eq!(editor.elements()[0].points.as_ptr(), points);
            assert!(editor.layers().unwrap().same_storage(&original));
        }
        for expected in [first, second, None] {
            assert!(editor.redo());
            assert_eq!(editor.crop_view(), expected);
        }
        assert_eq!(editor.retained_points, retained);
        assert_eq!(snapshot.crop_view(), second);
        assert_eq!(editor.elements()[0].points[0], Point { x: 2.0, y: 2.0 });
    }
    #[test]
    fn invalid_noop_and_draft_preserve_redo() {
        let mut editor = document();
        editor.crop_to(rect(10.0, 5.0, 20.0, 10.0)).unwrap();
        editor.undo();
        let revision = editor.revision();
        for selection in [
            rect(-1.0, 0.0, 10.0, 10.0),
            rect(60.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 0.0, 10.0),
            rect(f64::NAN, 0.0, 10.0, 10.0),
        ] {
            assert!(editor.crop_to(selection).is_err());
        }
        assert!(!editor.crop_to(rect(0.0, 0.0, 64.0, 32.0)).unwrap());
        assert!(!editor.reset_crop());
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(!editor.crop_to(rect(1.0, 1.0, 10.0, 10.0)).unwrap());
        editor.cancel();
        assert_eq!(editor.revision(), revision);
        assert!(editor.redo());
    }
    #[test]
    fn crop_interleaves_with_style_and_raster_history() {
        let mut editor = document();
        let original = editor.layers().unwrap().clone();
        let style = editor.elements()[0].style();
        editor.crop_to(rect(5.0, 5.0, 20.0, 20.0)).unwrap();
        let crop = editor.crop_view();
        editor.restyle(0, style.with_color(Color(0, 0, 0)));
        let mut candidate = original.clone();
        candidate.base = vec![0; 64 * 32 * 4].into();
        editor
            .replace_layers(editor.revision(), &original, candidate)
            .unwrap();
        assert!(editor.undo());
        assert_eq!(editor.crop_view(), crop);
        assert!(editor.layers().unwrap().same_storage(&original));
        assert!(editor.undo());
        assert_eq!(editor.elements()[0].style(), style);
        assert_eq!(editor.crop_view(), crop);
        assert!(editor.undo());
        assert!(editor.crop_view().is_none());
    }
}
