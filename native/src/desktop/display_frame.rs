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
    publisher: Option<Publisher>,
    pub naming: Option<super::capture_window_meta::Meta>,
    pub identity: std::sync::Arc<()>,
    source: UnitId,
    overlay_owner: Option<UnitId>,
    pub image: Image,
    pub editor: Option<Box<Editor>>,
    stamp: Option<DocumentStamp>,
    overlay: bool,
}
/// A UI-only view of the original producer. Do not keep its canvas, edits or
/// window alive, and never dereference a recycled HWND to check provenance.
struct Publisher {
    owner: std::rc::Weak<()>,
    canvas: std::rc::Weak<std::cell::RefCell<super::canvas::Canvas>>,
    inline:
        std::rc::Weak<std::cell::RefCell<Option<std::rc::Rc<super::text_input::inline::Session>>>>,
    version: DocumentStamp,
}
impl Frame {
    /// Upstream marks are not editable objects owned by the receiving pin.
    pub fn has_local_overlays(&self) -> bool {
        self.has_local_source()
            && self.editor.as_deref().is_some_and(|e| {
                !e.elements().is_empty() || e.layers().is_some_and(|l| l.annotations.is_some())
            })
    }
    /// Ordinary upstream vectors do not own the target's crop or appearance.
    /// A redaction, a presentation effect or an opaque raster overlay cannot
    /// be replaced with the unmarked natural source on a one-way send.
    pub fn needs_protected_pixels(&self) -> bool {
        use crate::editor::Tool;
        self.has_local_source()
            && self.editor.as_deref().is_some_and(|e| {
                e.layers().is_some_and(|l| l.annotations.is_some())
                    || e.elements().iter().any(|s| {
                        s.blur.is_some()
                            || matches!(s.tool, Tool::Blur | Tool::Mosaic | Tool::Magnifier)
                            || s.tool.spotlight()
                    })
            })
    }
    pub fn bind_local(&mut self, state: &super::State) {
        self.publisher = Some(Publisher {
            owner: state.lifetime.publication(),
            canvas: std::rc::Rc::downgrade(&state.canvas),
            inline: std::rc::Rc::downgrade(&state.inline_text),
            version: state.canvas.borrow().editor.document_stamp(),
        });
    }
    pub fn current_producer(&self) -> bool {
        self.publisher.as_ref().is_none_or(|p| {
            p.owner.upgrade().is_some()
                && p.inline
                    .upgrade()
                    .is_some_and(|c| c.try_borrow().is_ok_and(|edit| edit.is_none()))
                && p.canvas.upgrade().is_some_and(|c| {
                    c.try_borrow().is_ok_and(|canvas| {
                        p.version.matches(&canvas.editor)
                            && super::display_snapshot::locally_settled(&canvas)
                    })
                })
        })
    }
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
            publisher: None,
            naming: None,
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
            publisher: None,
            naming: None,
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
