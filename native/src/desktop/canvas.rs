//! Window-private D2D resources; draw only in WM_PAINT and discard GPU resources after device loss.
use super::image::Image;
use super::render_surface::RenderSurface;
use crate::editor::{Editor, Tool};
use crate::viewport::{Point, Viewport};
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "canvas_cpu_recovery_tests.rs"]
mod cpu_recovery_tests;
#[path = "canvas_creation_admission.rs"]
mod creation_admission;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "canvas_d3d12_tests.rs"]
mod d3d12_tests;
#[cfg(test)]
#[path = "canvas_gpu_reselect_tests.rs"]
mod gpu_reselect_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "canvas_manual_render_tests.rs"]
mod manual_render_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "canvas_recovery_window_tests.rs"]
mod recovery_window_tests;
#[path = "canvas_render_recovery.rs"]
mod render_recovery;
pub(in crate::desktop) use render_recovery::Destination as RenderChoice;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_visuals_tests.rs"]
mod extension_visuals_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "canvas_frame_style_tests.rs"]
mod frame_style_tests;
#[path = "canvas_submission.rs"]
mod submission;
#[path = "canvas_target.rs"]
mod target;
#[path = "canvas_zoom_snapshot.rs"]
mod zoom_snapshot;
pub(in crate::desktop) use submission::Pending;
#[cfg(test)]
#[path = "canvas_software_tests.rs"]
mod software_tests;
use windows::{
    core::Result,
    Win32::{
        Foundation::HWND,
        Graphics::Direct2D::{Common::*, *},
    },
};
#[path = "canvas_capture_source.rs"]
mod capture;
#[path = "canvas_display.rs"]
mod display;
#[cfg(test)]
#[path = "canvas_long_raster_tests.rs"]
mod long_raster_tests;
#[path = "canvas_minified.rs"]
pub(super) mod minified;
#[path = "canvas_minified_source.rs"]
pub(super) mod minified_source;
#[path = "canvas_raster.rs"]
mod raster;
pub use capture::CaptureSource;

struct Target {
    identity: std::rc::Rc<()>,
    effects: super::effect_cache::Cache,
    brush: ID2D1SolidColorBrush,
    raster: raster::Resources,
    surface: RenderSurface,
}

pub struct Canvas {
    pub(super) live_pin: bool,
    target: Option<Target>,
    creation_admission: creation_admission::Admission,
    recovery: render_recovery::Recovery,
    // Stay on the compatibility path after creation/runtime fallback; ordinary redraw/minimize must not reprobe the failed primary backend.
    primary: super::render_surface::Primary,
    compatibility_renderer: bool,
    software_renderer: bool,
    pub(super) factory: ID2D1Factory,
    _shared_factory: std::rc::Rc<ID2D1Factory>,
    pub image: Option<Image>,
    capture_source: Option<(crate::editor::DocumentStamp, CaptureSource)>,
    pub(super) blur_ready: Option<(
        crate::erase_layers::Layers,
        crate::blur_image::BlurImage,
        Option<crate::crop_geometry::Rect>,
        Option<crate::image_projection::Projection>,
    )>,
    pub viewport: Viewport,
    pub(super) client_size: Option<Point>,
    pub(super) crop_mode: bool,
    pub(super) crop_pending: Option<crate::editor::RasterRefreshPlan>,
    pub(super) applied_crop: Option<crate::crop_geometry::Rect>,
    pub(super) crop_drag: Option<super::crop_input::Drag>,
    pub drag: Option<Point>,
    pub(super) draft_pointer: Option<Point>,
    pub(super) motion_epoch: std::time::Instant,
    pub(super) message_clock: crate::message_clock::MessageClock,
    pub(super) last_pointer_time: f64,
    pub(super) path_drag: Option<super::path_handles::Drag>,
    pub(super) erase_gesture: Option<crate::erase_job::Request>,
    pub(super) erase_pending: Option<crate::erase_job::Request>,
    pub(super) erase_display: Option<crate::erase_job::Request>,
    pub(super) erase_annotations_only: bool,
    pub paints: u32,
    /// Count only newly submitted frames through the local full-image path, not zero-size, external-display, or thumbnail operations as received presentation.
    pub(super) confirmed_paints: u64,
    pub editor: Editor,
    pub tool: Tool,
    pub(super) selection_enabled: bool,
    pub pinned: bool,
    pub(super) window_opacity: f32,
    /// Baseline opacityNormal: output affects only the base image, not the opacity used by minified presentation.
    pub(super) normal_opacity: f32,
    pub(super) frame_style: crate::pin_frame_style::Style,
    pub(super) extension_views: Option<super::extension_visuals::Bound>,
    pub(super) minified: Option<minified::Presentation>,
    pub(super) minified_source: Option<minified_source::Source>,
    pub(super) minified_opacity: f32,
    pub(super) minified_restore: Option<crate::minified_transition::Transition>,
    pub(super) selected: Option<(usize, u64)>,
    pub(super) multi_selection: Option<(Vec<usize>, u64)>,
    pub(super) transform_own: bool,
    pub(super) move_axis: super::transform_mode::MoveAxis,
    pub(super) rotation_axis: crate::transform_gesture::Axis,
    pub(super) scale_axis: crate::transform_gesture::Axis,
    pub(super) rotation_drag: Option<super::rotation_handle::Drag>,
    pub(super) scale_drag: Option<super::scale_handle::Drag>,
    pub(super) scale_pending: Option<crate::editor::ResizePlan>,
    pub(super) scale_display_revision: Option<u64>,
    pub(super) selection_drag: Option<(Point, Point)>,
    pub(super) marquee: Option<super::marquee::Marquee>,
    pub(super) selection_effect_preview: Option<crate::editor::Stroke>,
    pub(super) inline_text_hidden: Option<usize>,
    pub(super) group_preview: Vec<(usize, crate::editor::Stroke)>,
    pub(super) group_translation: Option<super::multi_selection::GroupTranslation>,
    pub(super) tool_profiles: crate::tool_profiles::ToolProfiles,
}

thread_local! {
    static CANVAS_FACTORY: std::cell::RefCell<std::rc::Weak<ID2D1Factory>> =
        const { std::cell::RefCell::new(std::rc::Weak::new()) };
}

fn acquire_canvas_factory() -> Result<std::rc::Rc<ID2D1Factory>> {
    CANVAS_FACTORY.with(|slot| {
        if let Some(existing) = slot.borrow().upgrade() {
            return Ok(existing);
        }
        let created = std::rc::Rc::new(super::smoke_timing::run("canvas-factory", || unsafe {
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)
        })?);
        *slot.borrow_mut() = std::rc::Rc::downgrade(&created);
        Ok(created)
    })
}

impl Canvas {
    pub fn new(image: Option<Image>) -> Result<Self> {
        let editor = raster::editor(image.as_ref())?;
        let shared_factory = acquire_canvas_factory()?;
        Ok(Self {
            live_pin: false,
            target: None,
            creation_admission: Default::default(),
            recovery: Default::default(),
            primary: super::render_surface::Primary::SharedD3d11,
            compatibility_renderer: false,
            software_renderer: false,
            factory: (*shared_factory).clone(),
            _shared_factory: shared_factory,
            image,
            capture_source: None,
            blur_ready: None,
            viewport: Viewport::default(),
            client_size: None,
            crop_mode: false,
            crop_pending: None,
            applied_crop: None,
            crop_drag: None,
            drag: None,
            draft_pointer: None,
            motion_epoch: std::time::Instant::now(),
            message_clock: crate::message_clock::MessageClock::default(),
            last_pointer_time: 0.0,
            path_drag: None,
            erase_gesture: None,
            erase_pending: None,
            erase_display: None,
            erase_annotations_only: false,
            paints: 0,
            confirmed_paints: 0,
            editor,
            tool: Tool::Pan,
            selection_enabled: true,
            pinned: false,
            window_opacity: 1.0,
            normal_opacity: 1.0,
            frame_style: Default::default(),
            extension_views: None,
            minified: None,
            minified_source: None,
            minified_opacity: 0.9,
            minified_restore: None,
            selected: None,
            multi_selection: None,
            transform_own: false,
            move_axis: Default::default(),
            rotation_axis: Default::default(),
            scale_axis: Default::default(),
            rotation_drag: None,
            scale_drag: None,
            scale_pending: None,
            scale_display_revision: None,
            selection_drag: None,
            marquee: None,
            selection_effect_preview: None,
            inline_text_hidden: None,
            group_preview: Vec::new(),
            group_translation: None,
            tool_profiles: crate::tool_profiles::ToolProfiles::default(),
        })
    }

    pub fn discard_device(&mut self) {
        self.target = None;
    }

    /// Accept only fully decoded images; the caller prepares discard confirmation and pin hit-region transactions.
    pub fn replace_image(&mut self, image: Image) -> Result<()> {
        let editor = raster::editor(Some(&image))?;
        let style = self.editor.style();
        self.cancel_gesture();
        self.clear_scale_display();
        self.discard_device();
        self.minified_source = None;
        self.capture_source = None;
        self.image = Some(image);
        self.crop_mode = false;
        self.crop_pending = None;
        self.applied_crop = None;
        self.erase_display = None;
        self.blur_ready = None;
        self.selected = None;
        self.multi_selection = None;
        self.editor = editor;
        self.editor.set_style(style);
        self.viewport = Viewport::default();
        Ok(())
    }

    pub fn paint(&mut self, hwnd: HWND, width: u32, height: u32, dpi: u32) -> Result<()> {
        self.paint_display(hwnd, width, height, dpi, None)
    }

    pub(super) fn paint_display(
        &mut self,
        hwnd: HWND,
        width: u32,
        height: u32,
        dpi: u32,
        frame: Option<&super::display_frame::Frame>,
    ) -> Result<()> {
        self.ensure_render_admitted()?;
        self.client_size = Some(super::transform_handle_layout::logical_size(
            width, height, dpi,
        ));
        if frame.is_none() {
            self.validate_scale_display();
            self.discard_stale_group_translation();
            self.sync_raster_source();
        }
        if width == 0 || height == 0 {
            return Ok(());
        }
        if self.minified.is_none() {
            if let Some(views) = &self.extension_views {
                let size = self.client_size.unwrap();
                views.resize(self, frame, [size.x, size.y])?;
            }
        }
        // COM objects stay on the message thread; the pixel Vec outlives the synchronous CreateBitmap call.
        unsafe {
            let size = D2D_SIZE_U { width, height };
            if self.target.is_none() {
                let prepared = self.creation_admission.attempt(|| {
                    let surface = if self.software_renderer {
                        RenderSurface::software_mode(&self.factory, size, self.pinned)
                            .map_err(Into::into)
                    } else if self.compatibility_renderer {
                        RenderSurface::legacy(&self.factory, hwnd, size, dpi, self.pinned)
                    } else {
                        RenderSurface::new(
                            &self.factory,
                            hwnd,
                            size,
                            dpi,
                            self.pinned,
                            self.primary,
                        )
                    }?;
                    Target::prepare(surface, |target| {
                        target.CreateSolidColorBrush(
                            &D2D1_COLOR_F {
                                r: 0.96,
                                g: 0.88,
                                b: 0.24,
                                a: 1.0,
                            },
                            None,
                        )
                    })
                })?;
                if !matches!(prepared.surface, RenderSurface::D3d12(_)) {
                    self.primary = super::render_surface::Primary::SharedD3d11;
                }
                self.compatibility_renderer |= prepared.surface.is_compatibility();
                self.software_renderer |= prepared.surface.is_software();
                self.target = Some(prepared);
            }
            self.target
                .as_mut()
                .unwrap()
                .surface
                .set_opacity(self.window_opacity)?;
            if let Some(frame) = frame {
                if self.minified.is_some() {
                    return self.draw_minified_display(frame, size, dpi);
                }
                return self.draw_display(frame, size, dpi);
            }
            if self.minified.is_some() {
                return self.draw_minified(size, dpi);
            }
            let selected = self.selected_index();
            let (rotation_handle, scale_handle) = super::transform_handle_layout::positions(self);
            let group = self
                .multi_selection
                .as_ref()
                .filter(|_| {
                    self.selected_group()
                        .is_some_and(|indices| indices.len() > 1)
                })
                .map(|(indices, _)| indices);
            let preview = if self.selection_effect_preview.is_some() {
                None
            } else {
                selected.zip(self.selection_drag.map(|(_, offset)| offset))
            };
            let resources = self.target.as_mut().unwrap();
            resources.surface.resize(size)?;
            let target = resources.surface.resource_target();
            resources
                .raster
                .sync_frame(target, self.image.as_ref(), Some(&self.editor))?;
            target.SetDpi(dpi as f32, dpi as f32);
            let confirmed = resources.surface.draw(|target| {
                let background = if self.pinned {
                    D2D1_COLOR_F::default()
                } else {
                    D2D1_COLOR_F {
                        r: 0.07,
                        g: 0.08,
                        b: 0.08,
                        a: 1.0,
                    }
                };
                target.Clear(Some(&background));
                let frame = super::frame_style_draw::Frame::new(self.frame_style, size, dpi);
                let rounded = frame.clip(target, &self.factory)?;
                if let Some(image) = &self.image {
                    let raster::Geometry {
                        base,
                        clip,
                        projection,
                    } = raster::Geometry::new(image, Some(&self.editor), self.viewport);
                    let erasing = self
                        .erase_gesture
                        .as_ref()
                        .or(self.erase_display.as_ref())
                        .filter(|request| {
                            request.revision == self.editor.revision()
                                && self
                                    .editor
                                    .layers()
                                    .is_some_and(|layers| layers.same_storage(&request.source))
                        });
                    let clipped = super::render_frame::Clip::push(target, &clip);
                    resources.raster.draw_interleaved(
                        target,
                        &base,
                        &clip,
                        erasing,
                        (self.viewport, projection),
                        || frame.border_over_clip(target, &clip),
                    )?;
                    let drawing = if !self.group_preview.is_empty() {
                        super::annotations::draw_group_preview(
                            target,
                            &resources.brush,
                            &self.editor,
                            self.viewport,
                            &self.group_preview,
                            &mut resources.effects,
                        )
                    } else {
                        super::annotations::draw_preview_excluding(
                            target,
                            &resources.brush,
                            &self.editor,
                            self.viewport,
                            preview,
                            self.path_drag
                                .as_ref()
                                .map(|drag| (drag.index, &drag.preview))
                                .or_else(|| selected.zip(self.selection_effect_preview.as_ref())),
                            self.inline_text_hidden,
                            &mut resources.effects,
                        )
                    };
                    drop(clipped);
                    drawing?;
                    if let Some(views) = &self.extension_views {
                        views.draw(&self.editor, None, target, &resources.brush, (size, dpi))?;
                    }
                    drop(rounded);
                    if let Some(index) = selected {
                        let mut viewport = self.viewport;
                        if let Some((_, offset)) = preview {
                            viewport.origin.x += offset.x * viewport.scale;
                            viewport.origin.y += offset.y * viewport.scale;
                        }
                        let stroke = self
                            .path_drag
                            .as_ref()
                            .map(|drag| &drag.preview)
                            .or(self.selection_effect_preview.as_ref())
                            .or_else(|| {
                                self.group_preview
                                    .iter()
                                    .find(|(target, _)| *target == index)
                                    .map(|(_, stroke)| stroke)
                            })
                            .unwrap_or(&self.editor.elements()[index]);
                        super::selection::draw_outline(target, &resources.brush, stroke, viewport);
                        if self.rotation_drag.is_none() && self.scale_drag.is_none() {
                            super::path_handles::draw(target, &resources.brush, stroke, viewport);
                        }
                    }
                }
                if let Some(indices) = group {
                    for &index in indices {
                        super::selection::draw_outline(
                            target,
                            &resources.brush,
                            self.group_preview
                                .binary_search_by_key(&index, |(index, _)| *index)
                                .ok()
                                .map(|position| &self.group_preview[position].1)
                                .unwrap_or(&self.editor.elements()[index]),
                            self.viewport,
                        );
                    }
                }
                super::rotation_handle::draw(target, &resources.brush, rotation_handle);
                super::marquee::draw(
                    target,
                    &resources.brush,
                    self.marquee.as_ref(),
                    self.viewport,
                );
                super::crop_input::draw(
                    target,
                    &resources.brush,
                    self.crop_drag.as_ref(),
                    self.viewport,
                );
                super::scale_handle::draw(target, &resources.brush, scale_handle);
                super::erase_gesture::draw_cursor(
                    target,
                    &resources.brush,
                    self.erase_gesture.as_ref(),
                    self.viewport,
                );
                if self.live_pin {
                    super::live_pin_document::draw(target, &resources.brush, size, dpi);
                }
                Ok(())
            })?;
            if confirmed {
                self.confirmed_paints = self.confirmed_paints.saturating_add(1);
            }
            self.paints = self.paints.saturating_add(1);
            Ok(())
        }
    }
}
