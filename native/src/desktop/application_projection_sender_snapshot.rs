//! UI shares only authoritative pixels and committed edits; retained source must first match the active editor.
use super::*;
use crate::desktop::canvas::minified_source::Origin;
use crate::desktop::{canvas::Canvas, display_frame::Frame};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "application_projection_sender_provenance_tests.rs"]
mod provenance_tests;

pub(in crate::desktop::application) struct Snapshot {
    pub image: Image,
    pub editor: Editor,
    pub opacity: f32,
    pub kind: crate::projection_delivery::sender::Kind,
    pub geometry: Option<crate::desktop::export::projection::Geometry>,
    protected_pixels: Option<(Image, Editor)>,
    upstream_overlays: bool,
    pub source_logical: Option<crate::crop_geometry::Size>,
    rasterized_source: bool,
    pub natural_origin: Option<Origin>,
    // Relative to the actually selected pixel owner, not canonical historical crop; do not flip natural coordinates twice.
    pub legacy_pixels: Option<crate::crop_geometry::Rect>,
    pending_content: bool,
}
pub(super) struct Stamp {
    size: (u32, u32),
    pixels: std::sync::Weak<crate::pixels::Storage>,
    source_logical: Option<crate::crop_geometry::Size>,
    rasterized_source: bool,
    natural_origin: Option<(super::content::SourceStamp, crate::workspace_id::UnitId)>,
    legacy_pixels: Option<crate::crop_geometry::Rect>,
    pending_content: bool,
}
struct Selected {
    image: Image,
    logical: Option<crate::crop_geometry::Size>,
    rasterized_source: bool,
    origin: Option<Origin>,
    legacy_pixels: Option<crate::crop_geometry::Rect>,
    content: bool,
}
impl Snapshot {
    pub(super) fn capture(
        canvas: &Canvas,
        frame: Option<&Frame>,
        kind: crate::projection_delivery::sender::Kind,
        documents: &[crate::desktop::document_window::DocumentWindow],
    ) -> Option<Self> {
        if !crate::desktop::display_snapshot::settled(canvas) {
            return None;
        }
        // export_snapshot creates a new document identity; Source::matches must be checked before that.
        let Selected {
            image,
            logical: source_logical,
            rasterized_source,
            origin: natural_origin,
            legacy_pixels,
            content: pending_content,
        } = select(canvas, frame, documents)?;
        // Resolved annotations need not belong to the sender's editor. The
        // one-way path must not send their unmarked natural image instead.
        let protected_pixels = frame.filter(|f| f.needs_protected_pixels()).map(|f| {
            (
                f.image.clone(),
                f.editor
                    .as_ref()
                    .expect("resolved local marks")
                    .export_snapshot(),
            )
        });
        Some(Self {
            image,
            protected_pixels,
            upstream_overlays: frame.is_some_and(Frame::has_local_overlays),
            source_logical,
            rasterized_source,
            natural_origin,
            legacy_pixels,
            pending_content,
            // resolved's editor may belong upstream and can prove only the base image; it must not take ownership of target annotations/cropping.
            editor: canvas.editor.export_snapshot(),
            opacity: canvas.normal_opacity,
            kind,
            geometry: None,
        })
    }
    pub(in crate::desktop::application) fn display_only(&self) -> bool {
        self.upstream_overlays
    }
    pub(in crate::desktop::application) fn one_way(
        &self,
        source: &crate::session_source::Source,
        unit: &str,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> windows::core::Result<crate::projection_delivery::sender::Frame> {
        use crate::desktop::export;
        let flattened;
        let (image, input) = if let Some((image, editor)) = &self.protected_pixels {
            flattened = protected_source::compose(image, editor, cancel)?;
            // The working grid is already redacted. Never substitute retained
            // natural pixels or replay their historical edit baseline here.
            let input = export::projection::Input {
                logical: self.render_input().logical,
                content: true,
                ..Default::default()
            };
            (&flattened, input)
        } else {
            (&self.image, self.render_input())
        };
        export::projection::target_frame_with_input(
            source,
            unit,
            image,
            &self.editor,
            export::projection::Presentation {
                opacity: self.opacity,
                kind: self.kind,
                geometry: self.geometry,
            },
            input,
            cancel,
        )
    }
    pub(super) fn stamp(&self) -> Stamp {
        Stamp {
            size: (self.image.width, self.image.height),
            pixels: self.image.pixels.downgrade(),
            source_logical: self.source_logical,
            rasterized_source: self.rasterized_source,
            natural_origin: self.natural_origin.as_ref().map(|origin| {
                (
                    super::content::SourceStamp::capture(&origin.record),
                    origin.unit.clone(),
                )
            }),
            legacy_pixels: self.legacy_pixels,
            pending_content: self.pending_content,
        }
    }
    pub(in crate::desktop::application) fn render_input(
        &self,
    ) -> crate::desktop::export::projection::Input {
        use crate::desktop::export::projection::{Baseline, Input};
        Input {
            logical: self
                .editor
                .full_logical_frame()
                .map(|frame| crate::crop_geometry::Size {
                    width: frame.width,
                    height: frame.height,
                })
                .or_else(|| {
                    // Ordinary non-restored Editor has no logical_size, but its active base image still supplies the full logical frame.
                    // Do not fall back to canonical window dimensions that may not yet be saved, or use the cropped visible frame.
                    self.editor
                        .layers()
                        .map(|layers| crate::crop_geometry::Size {
                            width: layers.size.0 as f64,
                            height: layers.size.1 as f64,
                        })
                }),
            source_logical: self.source_logical,
            rasterized_source: self.rasterized_source,
            baseline: self.natural_origin.as_ref().map(|origin| Baseline {
                record: origin.record.clone(),
                unit: origin.unit.clone(),
            }),
            // Restoration already materialized flip; live flip replaces the base image and invalidates retained, so canonical flags must not be replayed.
            flip: (false, false),
            historical_erase: self.natural_origin.is_some(),
            content: self.pending_content,
            // Legacy crop without active Editor credentials must not be revived from canonical; it needs a separate restoration/reset contract.
            legacy_crop: false,
            legacy_pixels: self.legacy_pixels,
        }
    }
}

#[path = "application_projection_redacted_grid.rs"]
mod protected_source;
impl Stamp {
    pub(super) fn matches(
        &self,
        canvas: &Canvas,
        frame: Option<&Frame>,
        documents: &[crate::desktop::document_window::DocumentWindow],
    ) -> bool {
        select(canvas, frame, documents).is_some_and(
            |Selected {
                 image,
                 logical,
                 rasterized_source,
                 origin,
                 legacy_pixels,
                 content,
             }| {
                self.size == (image.width, image.height)
                    && self.pixels.ptr_eq(&image.pixels.downgrade())
                    && self.source_logical == logical
                    && self.rasterized_source == rasterized_source
                    && self.pending_content == content
                    && self.legacy_pixels == legacy_pixels
                    && match (&self.natural_origin, &origin) {
                        (None, None) => true,
                        (Some((record, unit)), Some(b)) => {
                            *unit == b.unit && record.matches(&b.record)
                        }
                        _ => false,
                    }
            },
        )
    }
}
fn select(
    canvas: &Canvas,
    frame: Option<&Frame>,
    documents: &[crate::desktop::document_window::DocumentWindow],
) -> Option<Selected> {
    if frame.is_some_and(|f| !f.current_producer()) {
        return None;
    }
    if let Some(frame) = frame.filter(|frame| frame.has_local_source()) {
        let source = documents
            .iter()
            // DocumentWindow is UI-thread-owned; query only existing HWNDs here, without dispatching messages.
            .find(|doc| frame.is_from(&doc.state.unit_id) && unsafe { doc.is_alive() })?;
        if !crate::desktop::display_snapshot::ready(&source.state) {
            return None;
        }
        let upstream = source.state.canvas.borrow();
        return select_local(frame, &source.state.unit_id, &upstream);
    }
    let (image, editor) = if let Some(frame) = frame {
        // resolved pixels/base images may belong upstream; the target's retained source must not replace them.
        (&frame.image, frame.editor.as_deref())
    } else {
        if let Some(source) = canvas
            .minified_source
            .as_ref()
            .filter(|source| source.matches(&canvas.editor))
        {
            return Some(Selected {
                image: source.image().clone(),
                logical: source.source_logical(&canvas.editor),
                rasterized_source: source.rasterized_crop(),
                origin: source.origin(&canvas.editor).cloned(),
                legacy_pixels: canvas
                    .editor
                    .crop_view()
                    .and_then(|view| view.legacy_pixel_region()),
                content: true,
            });
        }
        (canvas.image.as_ref()?, Some(&canvas.editor))
    };
    // Base-image erasure replaces layers.base while host image may remain old; never revive content from the host.
    let image = editor.and_then(Editor::layers).map_or_else(
        || image.clone(),
        |layers| Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        },
    );
    Some(Selected {
        image,
        logical: None,
        rasterized_source: editor.is_some_and(is_restored_grid),
        origin: None,
        legacy_pixels: editor
            .and_then(Editor::raster_crop_view)
            .and_then(|view| view.legacy_pixel_region()),
        content: frame.is_some() || canvas.editor.layers().is_none(),
    })
}

fn select_local(
    frame: &Frame,
    unit: &crate::workspace_id::UnitId,
    upstream: &Canvas,
) -> Option<Selected> {
    let image = frame.natural_source(unit, upstream)?;
    let retained = upstream
        .minified_source
        .as_ref()
        .filter(|source| source.matches(&upstream.editor));
    // source_logical belongs to the natural source, not the target frame. The rendering entry point uses current target logical dimensions to normalize crop per axis,
    // then maps back to original sourceSize; nonuniform target resizing must preserve original-image density credentials.
    Some(Selected {
        image,
        logical: frame.natural_source_logical(unit, upstream),
        rasterized_source: retained.map_or_else(
            || is_restored_grid(&upstream.editor),
            |source| source.rasterized_crop(),
        ),
        origin: frame.natural_source_origin(unit, upstream),
        legacy_pixels: if retained.is_some() {
            upstream.editor.crop_view()
        } else {
            upstream.editor.raster_crop_view()
        }
        .and_then(|view| view.legacy_pixel_region()),
        content: retained.is_some(),
    })
}

fn is_restored_grid(editor: &Editor) -> bool {
    editor.layers().is_some()
        && editor.full_logical_frame().is_some()
        && editor
            .crop_view()
            .is_none_or(|view| view.legacy_pixel_region().is_none())
}
