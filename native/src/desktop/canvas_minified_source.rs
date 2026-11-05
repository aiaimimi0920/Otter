//! 恢复原图与编辑底图的关联；标注变化不退休原图，底图/文档替换后不得复用旧来源。
use super::{Editor, Image};
use crate::{
    crop_geometry::{Rect, Size},
    editor::DocumentStamp,
    pixels::Pixels,
};

struct Backing {
    pixels: Pixels,
    size: (u32, u32),
    logical_frame: Option<Rect>,
}
/// 自然像素对应的恢复基线；共享原文 owner，不复制会话或历史数组。
#[derive(Clone)]
pub(in crate::desktop) struct Origin {
    pub record: crate::session_source::Source,
    pub unit: crate::workspace_id::UnitId,
}
impl Origin {
    #[cfg(test)]
    pub(in crate::desktop) fn same(&self, other: &Self) -> bool {
        self.unit == other.unit && self.record.same_storage(&other.record)
    }
}
pub(in crate::desktop) struct Source {
    image: Image,
    document: DocumentStamp,
    backing: Option<Backing>,
    source_logical: Option<Size>,
    origin: Option<Origin>,
}
impl Source {
    pub(in crate::desktop) fn new(image: Image, editor: &Editor) -> Self {
        Self {
            image,
            document: editor.document_stamp(),
            source_logical: None,
            origin: None,
            backing: editor.layers().map(|layers| Backing {
                pixels: layers.base.clone(),
                size: layers.size,
                logical_frame: editor.full_logical_frame(),
            }),
        }
    }
    pub(in crate::desktop) fn image(&self) -> &Image {
        &self.image
    }
    /// 仅恢复路径提供原逻辑源框；普通替换/同步不从像素尺寸猜测此映射。
    pub(in crate::desktop) fn with_source_logical(mut self, logical: Size) -> Self {
        // 与 crop_geometry 的逻辑坐标合同一致；不是纹理大小，不套用 8192 像素门。
        self.source_logical = (logical.width.is_finite()
            && logical.height.is_finite()
            && logical.width > 0.0
            && logical.height > 0.0
            && logical.width <= 1_000_000.0
            && logical.height <= 1_000_000.0)
            .then_some(logical);
        self
    }
    pub(in crate::desktop) fn source_logical(&self, editor: &Editor) -> Option<Size> {
        self.matches(editor)
            .then_some(self.source_logical)
            .flatten()
    }
    pub(in crate::desktop) fn with_origin(mut self, origin: Origin) -> Self {
        self.origin = Some(origin);
        self
    }
    pub(in crate::desktop) fn origin(&self, editor: &Editor) -> Option<&Origin> {
        self.matches(editor)
            .then_some(self.origin.as_ref())
            .flatten()
    }
    pub(in crate::desktop) fn belongs_to(&self, editor: &Editor) -> bool {
        self.document.same_document(editor)
    }
    pub(in crate::desktop) fn matches(&self, editor: &Editor) -> bool {
        if !self.belongs_to(editor) {
            return false;
        }
        match (&self.backing, editor.layers()) {
            (None, None) => true,
            (Some(backing), Some(layers)) => {
                backing.size == layers.size
                    && backing.pixels.same_storage(&layers.base)
                    && backing.logical_frame == editor.full_logical_frame()
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::Tool,
        erase_layers::{Layers, Mode},
        viewport::Point,
    };
    fn image() -> Image {
        Image {
            width: 8,
            height: 4,
            pixels: vec![255; 128].into(),
        }
    }
    fn editor(image: &Image) -> Editor {
        Editor::with_layers(Layers {
            size: (8, 4),
            base: image.pixels.clone(),
            annotations: Some(vec![255; 128].into()),
            annotation_size: None,
        })
        .unwrap()
    }
    #[test]
    fn vector_overlay_and_crop_changes_keep_original_but_base_changes_retire_it() {
        let image = image();
        let mut editor = editor(&image);
        let logical = Size {
            width: 4.0,
            height: 2.0,
        };
        let origin = Origin {
            record: br#"{"stickers":[],"links":[]}"#.to_vec().into(),
            unit: crate::workspace_id::UnitId::parse("u").unwrap(),
        };
        let source = Source::new(image.clone(), &editor)
            .with_source_logical(logical)
            .with_origin(origin.clone());
        assert!(source.origin(&editor).unwrap().same(&origin));
        assert_eq!(source.source_logical(&editor), Some(logical));
        editor.begin(Tool::Brush, Point { x: 2.0, y: 2.0 }).unwrap();
        editor.commit();
        assert!(source.matches(&editor));
        let before = editor.layers().unwrap().clone();
        let next = before
            .erase(
                Mode::RasterAnnotations,
                &[Point { x: 2.0, y: 2.0 }],
                4.0,
                || false,
            )
            .unwrap();
        editor
            .replace_layers(editor.revision(), &before, next)
            .unwrap();
        assert!(source.matches(&editor));
        editor
            .crop_to(Rect {
                x: 1.0,
                y: 1.0,
                width: 4.0,
                height: 2.0,
            })
            .unwrap();
        assert!(source.matches(&editor));
        editor.reset_crop();
        assert!(source.matches(&editor));
        assert_eq!(source.source_logical(&editor), Some(logical));
        let before = editor.layers().unwrap().clone();
        let next = before
            .erase(Mode::Content, &[Point { x: 2.0, y: 2.0 }], 4.0, || false)
            .unwrap();
        editor
            .replace_layers(editor.revision(), &before, next)
            .unwrap();
        assert!(!source.matches(&editor));
        assert_eq!(source.source_logical(&editor), None);
        assert!(source.origin(&editor).is_none());
        assert!(source.image().pixels.same_storage(&image.pixels));
        editor.undo();
        assert!(source.matches(&editor));
        assert_eq!(source.source_logical(&editor), Some(logical));
        assert!(source.origin(&editor).unwrap().same(&origin));
        assert!(!source.matches(&editor.export_snapshot()));
    }
    #[test]
    fn ordinary_sources_do_not_infer_mapping_from_pixels_and_invalid_frames_are_rejected() {
        let image = image();
        let editor = editor(&image);
        let original = Size {
            width: 16384.0,
            height: 16384.0,
        };
        assert_eq!(
            Source::new(image.clone(), &editor)
                .with_source_logical(original)
                .source_logical(&editor),
            Some(original)
        );
        assert_eq!(
            Source::new(image.clone(), &editor).source_logical(&editor),
            None
        );
        for logical in [
            Size {
                width: f64::NAN,
                height: 2.0,
            },
            Size {
                width: 0.0,
                height: 2.0,
            },
            Size {
                width: 1_000_001.0,
                height: 2.0,
            },
        ] {
            assert_eq!(
                Source::new(image.clone(), &editor)
                    .with_source_logical(logical)
                    .source_logical(&editor),
                None
            );
        }
    }
    #[test]
    fn host_sync_releases_stale_source_even_without_another_minify_request() {
        let image = image();
        let mut canvas = super::super::Canvas::new(Some(image.clone())).unwrap();
        canvas.minified_source = Some(Source::new(image.clone(), &canvas.editor));
        let before = canvas.editor.layers().unwrap().clone();
        let next = before
            .erase(Mode::Content, &[Point { x: 2.0, y: 2.0 }], 4.0, || false)
            .unwrap();
        canvas
            .editor
            .replace_layers(canvas.editor.revision(), &before, next)
            .unwrap();
        canvas.sync_raster_source();
        assert!(!canvas
            .minified_source
            .as_ref()
            .unwrap()
            .matches(&canvas.editor));
        canvas.editor.undo();
        canvas.sync_raster_source();
        assert!(canvas
            .minified_source
            .as_ref()
            .unwrap()
            .matches(&canvas.editor));
        canvas.editor = editor(&image);
        canvas.sync_raster_source();
        assert!(canvas.minified_source.is_none());
        canvas.minified_source = Some(Source::new(image.clone(), &canvas.editor));
        canvas.replace_image(image).unwrap();
        assert!(canvas.minified_source.is_none());
    }
}
