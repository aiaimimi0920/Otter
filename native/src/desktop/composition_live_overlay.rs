//! Draw committed annotations over original Live frames; cache only target-private resources without modifying capture textures.
use super::*;
use crate::{
    editor::Editor,
    viewport::{Point, Viewport},
};
use std::{cell::RefCell, rc::Rc};

pub(super) struct Overlay {
    pub editor: Rc<Editor>,
    brush: ID2D1SolidColorBrush,
    annotations: Option<ID2D1Bitmap>,
    effects: RefCell<crate::desktop::effect_cache::Cache>,
}
impl Overlay {
    pub fn new(target: &ID2D1RenderTarget, editor: Rc<Editor>) -> Result<Self> {
        unsafe {
            let brush = target.CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)?;
            let annotations = editor
                .layers()
                .and_then(|layer| {
                    layer
                        .annotations
                        .as_ref()
                        .map(|pixels| (layer.annotation_dimensions(), pixels))
                })
                .map(|((width, height), pixels)| {
                    target.CreateBitmap(
                        D2D_SIZE_U { width, height },
                        Some(pixels.as_ptr().cast()),
                        width * 4,
                        &D2D1_BITMAP_PROPERTIES {
                            pixelFormat: D2D1_PIXEL_FORMAT {
                                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                            },
                            dpiX: 96.0,
                            dpiY: 96.0,
                        },
                    )
                })
                .transpose()?;
            Ok(Self {
                editor,
                brush,
                annotations,
                effects: RefCell::default(),
            })
        }
    }
    pub unsafe fn draw(
        &self,
        target: &ID2D1RenderTarget,
        bitmap: &ID2D1Bitmap1,
        size: D2D_SIZE_U,
        source_size: [u32; 2],
    ) -> Result<()> {
        let e = &self.editor;
        let (bounds, mapping, raster) = super::edit_view::geometry(e);
        let scale =
            (size.width as f64 / bounds.width).min(size.height as f64 / bounds.height) as f32;
        let viewport = Viewport {
            scale,
            origin: Point {
                x: (size.width as f32 - bounds.width as f32 * scale) / 2.0
                    - bounds.x as f32 * scale,
                y: (size.height as f32 - bounds.height as f32 * scale) / 2.0
                    - bounds.y as f32 * scale,
            },
        };
        let rect = |r: crate::crop_geometry::Rect| D2D_RECT_F {
            left: viewport.origin.x + r.x as f32 * scale,
            top: viewport.origin.y + r.y as f32 * scale,
            right: viewport.origin.x + (r.x + r.width) as f32 * scale,
            bottom: viewport.origin.y + (r.y + r.height) as f32 * scale,
        };
        let clip = crate::desktop::render_frame::Clip::push(target, &rect(bounds));
        let destination = rect(mapping.target);
        let mirror = e.mirror_state();
        let source_x = if mirror.x {
            raster[0] - mapping.source.x - mapping.source.width
        } else {
            mapping.source.x
        };
        let source_y = if mirror.y {
            raster[1] - mapping.source.y - mapping.source.height
        } else {
            mapping.source.y
        };
        let source = D2D_RECT_F {
            left: (source_x / raster[0] * source_size[0] as f64) as f32,
            top: (source_y / raster[1] * source_size[1] as f64) as f32,
            right: ((source_x + mapping.source.width) / raster[0] * source_size[0] as f64) as f32,
            bottom: ((source_y + mapping.source.height) / raster[1] * source_size[1] as f64) as f32,
        };
        let mut original = Default::default();
        target.GetTransform(&mut original);
        let mut flipped = original;
        if mirror.x {
            flipped.M11 = -flipped.M11;
            flipped.M31 += destination.left + destination.right;
        }
        if mirror.y {
            flipped.M22 = -flipped.M22;
            flipped.M32 += destination.top + destination.bottom;
        }
        target.SetTransform(&flipped);
        target.DrawBitmap(
            bitmap,
            Some(&destination),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            Some(&source),
        );
        target.SetTransform(&original);
        if let Some(layer) = &self.annotations {
            let dimensions = e.layers().unwrap().annotation_dimensions();
            let destination = rect(crate::crop_geometry::Rect {
                width: dimensions.0 as f64,
                height: dimensions.1 as f64,
                x: 0.0,
                y: 0.0,
            });
            target.DrawBitmap(
                layer,
                Some(&destination),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
        crate::desktop::annotations::draw_committed(
            target,
            &self.brush,
            e,
            viewport,
            &mut self.effects.borrow_mut(),
        )?;
        drop(clip);
        Ok(())
    }
}
