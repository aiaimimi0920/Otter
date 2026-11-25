//! Resolved display snapshots are independent of document base images; pixels are shared and local sources preserve editing information rather than discarding annotations.
use super::image::Image;
use crate::{
    editor::{DocumentStamp, Editor},
    workspace_id::UnitId,
};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
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
    /// Only uncomposited local sources can trace natural pixels; official outputs/target overlays cannot use this to replace sources.
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
        // Validate active-source document revision/base image before accessing retained; stale resolved snapshots must not revive natural sources.
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
