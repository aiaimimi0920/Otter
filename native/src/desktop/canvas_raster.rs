//! Target-private raster resources; update each layer by shared identity without recreating render targets or other effect caches.
use super::*;
use crate::{erase_layers::Layers, pixels::Pixels};
use windows::{
    core::Error,
    Win32::{Foundation::E_INVALIDARG, Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM},
};
#[path = "canvas_raster_projection.rs"]
mod projection;

pub(super) fn editor(image: Option<&Image>) -> Result<Editor> {
    image
        .map(|image| {
            Editor::with_layers(Layers {
                annotation_size: None,
                size: (image.width, image.height),
                base: image.pixels.clone(),
                annotations: None,
            })
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid raster document"))
        })
        .unwrap_or_else(|| Ok(Editor::default()))
}

impl Canvas {
    /// Update the compatibility host image view; share CPU pixels and synchronize GPU layers independently on the next paint.
    pub(in crate::desktop) fn sync_raster_source(&mut self) {
        if self
            .capture_source
            .as_ref()
            .is_some_and(|(stamp, _)| !stamp.same_document(&self.editor))
        {
            self.capture_source = None;
        }
        if self
            .minified_source
            .as_ref()
            .is_some_and(|source| !source.belongs_to(&self.editor))
        {
            self.minified_source = None;
        }
        let Some(layers) = self.editor.layers() else {
            self.image = None;
            self.blur_ready = None;
            return;
        };
        if self
            .blur_ready
            .as_ref()
            .is_some_and(|(source, _, frame, base_projection)| {
                !source.same_storage(layers)
                    || *frame != self.editor.annotation_frame()
                    || *base_projection != self.editor.raster_base_projection()
            })
        {
            self.blur_ready = None;
        }
        if self.image.as_ref().is_some_and(|image| {
            (image.width, image.height) == layers.size && image.pixels.same_storage(&layers.base)
        }) {
            return;
        }
        self.image = Some(Image {
            width: layers.size.0,
            height: layers.size.1,
            pixels: layers.base.clone(),
        });
        self.blur_ready = None;
    }
}

/// Keep the base projection and annotation clip in the same viewport for local and mirrored pins.
pub(super) struct Geometry {
    pub(super) base: D2D_RECT_F,
    pub(super) clip: D2D_RECT_F,
    pub(super) projection: Option<crate::image_projection::Projection>,
}
impl Geometry {
    pub(super) fn new(image: &Image, editor: Option<&Editor>, viewport: Viewport) -> Self {
        let Viewport { origin, scale, .. } = viewport;
        let to_display = |r: crate::crop_geometry::Rect| D2D_RECT_F {
            left: origin.x + r.x as f32 * scale,
            top: origin.y + r.y as f32 * scale,
            right: origin.x + (r.x + r.width) as f32 * scale,
            bottom: origin.y + (r.y + r.height) as f32 * scale,
        };
        let projection = editor.and_then(Editor::raster_base_projection);
        let base = projection.map_or_else(
            || D2D_RECT_F {
                left: origin.x,
                top: origin.y,
                right: origin.x + image.width as f32 * scale,
                bottom: origin.y + image.height as f32 * scale,
            },
            |mapping| to_display(mapping.target),
        );
        let clip = editor
            .and_then(Editor::annotation_frame)
            .map_or(base, to_display);
        Self {
            base,
            clip,
            projection,
        }
    }
}

struct Bitmap {
    pixels: Pixels,
    size: (u32, u32),
    bitmap: ID2D1Bitmap,
}
#[derive(Default)]
pub(super) struct Resources {
    base: Option<Bitmap>,
    annotations: Option<Bitmap>,
    preview: crate::desktop::erase_preview::Cache,
}

fn matches(stored: &Option<Bitmap>, pixels: Option<&Pixels>, size: (u32, u32)) -> bool {
    match (stored, pixels) {
        (None, None) => true,
        (Some(stored), Some(pixels)) => stored.size == size && stored.pixels.same_storage(pixels),
        _ => false,
    }
}
unsafe fn upload(
    target: &ID2D1RenderTarget,
    pixels: Option<&Pixels>,
    size: (u32, u32),
) -> Result<Option<Bitmap>> {
    pixels
        .map(|pixels| {
            if crate::viewport::checked_image_bytes(size.0, size.1) != Some(pixels.len()) {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Invalid raster texture dimensions",
                ));
            }
            let bitmap = target.CreateBitmap(
                D2D_SIZE_U {
                    width: size.0,
                    height: size.1,
                },
                Some(pixels.as_ptr().cast()),
                size.0 * 4,
                &D2D1_BITMAP_PROPERTIES {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                },
            )?;
            Ok(Bitmap {
                pixels: pixels.clone(),
                size,
                bitmap,
            })
        })
        .transpose()
}
impl Resources {
    pub(super) unsafe fn sync_frame(
        &mut self,
        target: &ID2D1RenderTarget,
        image: Option<&Image>,
        editor: Option<&Editor>,
    ) -> Result<()> {
        let layers = editor.and_then(Editor::layers);
        self.sync_sized(
            target,
            image,
            layers.and_then(|layers| layers.annotations.as_ref()),
            layers.map_or((0, 0), Layers::annotation_dimensions),
        )
    }

    #[cfg(test)]
    pub(super) unsafe fn sync(
        &mut self,
        target: &ID2D1RenderTarget,
        image: Option<&Image>,
        annotations: Option<&Pixels>,
    ) -> Result<()> {
        self.sync_sized(
            target,
            image,
            annotations,
            image.map_or((0, 0), |image| (image.width, image.height)),
        )
    }
    pub(super) unsafe fn sync_sized(
        &mut self,
        target: &ID2D1RenderTarget,
        image: Option<&Image>,
        annotations: Option<&Pixels>,
        annotation_size: (u32, u32),
    ) -> Result<()> {
        let size = image.map_or((0, 0), |image| (image.width, image.height));
        let base = image.map(|image| &image.pixels);
        let base_changed = !matches(&self.base, base, size);
        let annotations_changed = !matches(&self.annotations, annotations, annotation_size);
        // Swap only after both layer uploads succeed; retain old frame resources on failure, without a partially replaced intermediate state.
        let new_base = if base_changed {
            upload(target, base, size)?
        } else {
            None
        };
        let new_annotations = if annotations_changed {
            upload(target, annotations, annotation_size)?
        } else {
            None
        };
        if base_changed {
            self.base = new_base;
        }
        if annotations_changed {
            self.annotations = new_annotations;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) unsafe fn draw(
        &self,
        target: &ID2D1RenderTarget,
        rect: &D2D_RECT_F,
        annotation_rect: &D2D_RECT_F,
    ) {
        for (layer, destination) in [(&self.base, rect), (&self.annotations, annotation_rect)] {
            let Some(layer) = layer else {
                continue;
            };
            target.DrawBitmap(
                &layer.bitmap,
                Some(destination),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
    }
    #[cfg(test)]
    pub(super) unsafe fn draw_erasing(
        &mut self,
        target: &ID2D1RenderTarget,
        rect: &D2D_RECT_F,
        annotation_rect: &D2D_RECT_F,
        request: Option<&crate::erase_job::Request>,
        viewport: Viewport,
    ) -> Result<()> {
        self.draw_erasing_projected(target, rect, annotation_rect, request, viewport, None)
    }
    #[cfg(test)]
    pub(super) unsafe fn draw_erasing_projected(
        &mut self,
        target: &ID2D1RenderTarget,
        rect: &D2D_RECT_F,
        annotation_rect: &D2D_RECT_F,
        request: Option<&crate::erase_job::Request>,
        viewport: Viewport,
        mapping: Option<crate::image_projection::Projection>,
    ) -> Result<()> {
        self.draw_interleaved(
            target,
            rect,
            annotation_rect,
            request,
            (viewport, mapping),
            || Ok(()),
        )
    }
    pub(super) unsafe fn draw_interleaved(
        &mut self,
        target: &ID2D1RenderTarget,
        rect: &D2D_RECT_F,
        annotation_rect: &D2D_RECT_F,
        request: Option<&crate::erase_job::Request>,
        projection: (Viewport, Option<crate::image_projection::Projection>),
        between: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let (viewport, mapping) = projection;
        let Some(request) = request else {
            self.preview.clear();
            if let Some(base) = &self.base {
                projection::draw(target, base, rect, mapping, viewport);
            }
            between()?;
            if let Some(layer) = &self.annotations {
                projection::draw(target, layer, annotation_rect, None, viewport);
            }
            return Ok(());
        };
        let bounds = D2D_RECT_F {
            left: rect.left.min(annotation_rect.left),
            top: rect.top.min(annotation_rect.top),
            right: rect.right.max(annotation_rect.right),
            bottom: rect.bottom.max(annotation_rect.bottom),
        };
        let mask = self.preview.geometry(target, &bounds, request, viewport)?;
        // Match the commit kernel: erase each layer separately, then source-over. Compositing before masking incorrectly weakens two-layer soft edges.
        let mut between = Some(between);
        for (i, (bitmap, erase, destination, projection)) in [
            (
                &self.base,
                request.mode == crate::erase_layers::Mode::Content,
                rect,
                mapping,
            ),
            (&self.annotations, true, annotation_rect, None),
        ]
        .into_iter()
        .enumerate()
        {
            if i == 1 {
                between.take().unwrap()()?;
            }
            let Some(bitmap) = bitmap else {
                continue;
            };
            let _layer = erase.then(|| {
                crate::desktop::annotation_opacity::push_mask(target, bounds, mask.clone())
            });
            projection::draw(target, bitmap, destination, projection, viewport);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "canvas_raster_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canvas_raster_bounded_tests.rs"]
mod bounded_tests;
#[cfg(test)]
#[path = "erase_blend_tests.rs"]
mod erase_blend_tests;
