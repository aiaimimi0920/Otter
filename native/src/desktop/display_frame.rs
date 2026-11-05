//! 解析后的显示快照独立于文档底图；像素共享，本地来源保留编辑信息而非丢弃标注。
use super::image::Image;
use crate::{
    editor::{DocumentStamp, Editor},
    workspace_id::UnitId,
};
#[cfg(test)]
#[path = "display_frame_source_tests.rs"]
mod source_tests;
pub(super) struct Frame {
    pub identity: std::sync::Arc<()>,
    source: UnitId,
    overlay_owner: Option<UnitId>,
    pub image: Image,
    pub editor: Option<Box<Editor>>,
    stamp: Option<DocumentStamp>,
    overlay: bool,
}
impl Frame {
    /// 只有未合成的本地来源可追溯自然像素；正式输出与目标覆盖层不能借此换源。
    pub(in crate::desktop) fn has_local_source(&self) -> bool {
        !self.overlay && self.stamp.is_some()
    }
    pub(in crate::desktop) fn natural_source(
        &self,
        unit: &UnitId,
        canvas: &super::canvas::Canvas,
    ) -> Option<Image> {
        if !self.has_local_source() {
            return None;
        }
        let image = canvas.editor.layers().map_or_else(
            || canvas.image.clone(),
            |layers| {
                Some(Image {
                    width: layers.size.0,
                    height: layers.size.1,
                    pixels: layers.base.clone(),
                })
            },
        )?;
        // 先验证活动来源的文档版本/底图，再访问 retained。旧 resolved 不得复活自然源。
        if !self.matches(unit, &image, Some(&canvas.editor), false) {
            return None;
        }
        Some(
            canvas
                .minified_source
                .as_ref()
                .filter(|source| source.matches(&canvas.editor))
                .map_or(image, |source| source.image().clone()),
        )
    }
    pub(in crate::desktop) fn natural_source_logical(
        &self,
        unit: &UnitId,
        canvas: &super::canvas::Canvas,
    ) -> Option<crate::crop_geometry::Size> {
        self.natural_source(unit, canvas)?;
        canvas
            .minified_source
            .as_ref()?
            .source_logical(&canvas.editor)
    }
    pub(in crate::desktop) fn natural_source_origin(
        &self,
        unit: &UnitId,
        canvas: &super::canvas::Canvas,
    ) -> Option<super::canvas::minified_source::Origin> {
        self.natural_source(unit, canvas)?;
        canvas
            .minified_source
            .as_ref()?
            .origin(&canvas.editor)
            .cloned()
    }
    pub fn is_from(&self, source: &UnitId) -> bool {
        self.source == *source
    }
    pub fn has_overlay_owner(&self, owner: &UnitId) -> bool {
        self.overlay_owner.as_ref() == Some(owner)
    }
    pub fn capture(source: UnitId, image: Image, editor: Option<&Editor>) -> Self {
        Self::capture_with(source, image, editor, false)
    }
    pub fn capture_overlay(
        source: UnitId,
        owner: UnitId,
        image: Image,
        editor: &Editor,
    ) -> Result<Self, crate::editor::EditError> {
        let snapshot = editor.export_overlay_for_size((image.width, image.height), |text| {
            unsafe { super::annotation_text::measure(text) }
                .map_err(|_| crate::editor::EditError::InvalidPoint)
        })?;
        Ok(Self {
            overlay_owner: Some(owner),
            overlay: true,
            identity: std::sync::Arc::new(()),
            source,
            image,
            stamp: Some(editor.document_stamp()),
            editor: Some(Box::new(snapshot)),
        })
    }
    fn capture_with(source: UnitId, image: Image, editor: Option<&Editor>, overlay: bool) -> Self {
        Self {
            overlay_owner: None,
            overlay,
            identity: std::sync::Arc::new(()),
            source,
            image,
            stamp: editor.map(Editor::document_stamp),
            editor: editor.map(|editor| {
                Box::new(if overlay {
                    editor.export_overlay_snapshot()
                } else {
                    editor.export_snapshot()
                })
            }),
        }
    }
    pub fn matches(
        &self,
        source: &UnitId,
        image: &Image,
        editor: Option<&Editor>,
        overlay: bool,
    ) -> bool {
        self.overlay == overlay
            && self.source == *source
            && self.image.width == image.width
            && self.image.height == image.height
            && self.image.pixels.same_storage(&image.pixels)
            && match (&self.stamp, editor) {
                (None, None) => self.editor.is_none(),
                (Some(stamp), Some(editor)) => stamp.matches(editor),
                _ => false,
            }
    }
}
