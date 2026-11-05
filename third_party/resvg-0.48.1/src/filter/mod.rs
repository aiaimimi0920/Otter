// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::rc::Rc;

use rgb::{FromSlice, RGBA8};
use tiny_skia::IntRect;
use usvg::{ApproxEqUlps, ApproxZeroUlps};

mod color_matrix;
mod color_transfer;
mod component_transfer;
mod component_transfer_color;
mod composite;
mod convolve_matrix;
mod displacement_map;
mod gaussian_blur;
mod joint_gaussian;
mod lighting;
mod matrix_premultiplied;
mod morphology;
mod result_liveness;
mod scaled_frame;
mod small_gaussian_2d;
mod turbulence;
pub(crate) mod work_budget;

// TODO: apply single primitive filters in-place

/// An image reference.
///
/// Image pixels should be stored in RGBA order.
///
/// Some filters will require premultiplied channels, some not.
/// See specific filter documentation for details.
#[derive(Clone, Copy)]
pub struct ImageRef<'a> {
    data: &'a [RGBA8],
    width: u32,
    height: u32,
}

impl<'a> ImageRef<'a> {
    /// Creates a new image reference.
    ///
    /// Doesn't clone the provided data.
    #[inline]
    pub fn new(width: u32, height: u32, data: &'a [RGBA8]) -> Self {
        ImageRef {
            data,
            width,
            height,
        }
    }

    #[inline]
    fn alpha_at(&self, x: u32, y: u32) -> i16 {
        self.data[(self.width * y + x) as usize].a as i16
    }
}

/// A mutable `ImageRef` variant.
pub struct ImageRefMut<'a> {
    data: &'a mut [RGBA8],
    width: u32,
    height: u32,
}

impl<'a> ImageRefMut<'a> {
    /// Creates a new mutable image reference.
    ///
    /// Doesn't clone the provided data.
    #[inline]
    pub fn new(width: u32, height: u32, data: &'a mut [RGBA8]) -> Self {
        ImageRefMut {
            data,
            width,
            height,
        }
    }

    #[inline]
    fn pixel_at(&self, x: u32, y: u32) -> RGBA8 {
        self.data[(self.width * y + x) as usize]
    }

    #[inline]
    fn pixel_at_mut(&mut self, x: u32, y: u32) -> &mut RGBA8 {
        &mut self.data[(self.width * y + x) as usize]
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    InvalidRegion,
    NoResults,
    InvalidCoordinates,
    WorkBudgetExceeded,
}

trait PixmapExt: Sized {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error>;
    fn copy_region(&self, region: IntRect) -> Result<tiny_skia::Pixmap, Error>;
    fn into_srgb(&mut self);
    fn into_linear_rgb(&mut self);
}

impl PixmapExt for tiny_skia::Pixmap {
    fn try_create(width: u32, height: u32) -> Result<tiny_skia::Pixmap, Error> {
        tiny_skia::Pixmap::new(width, height).ok_or(Error::InvalidRegion)
    }

    fn copy_region(&self, region: IntRect) -> Result<tiny_skia::Pixmap, Error> {
        let rect = IntRect::from_xywh(region.x(), region.y(), region.width(), region.height())
            .ok_or(Error::InvalidRegion)?;
        self.clone_rect(rect).ok_or(Error::InvalidRegion)
    }

    fn into_srgb(&mut self) {
        color_transfer::convert(self.data_mut().as_rgba_mut(), false);
    }

    fn into_linear_rgb(&mut self) {
        color_transfer::convert(self.data_mut().as_rgba_mut(), true);
    }
}

/// Multiplies provided pixels alpha.
fn multiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f32 / 255.0;
        p.b = (p.b as f32 * a + 0.5) as u8;
        p.g = (p.g as f32 * a + 0.5) as u8;
        p.r = (p.r as f32 * a + 0.5) as u8;
    }
}

/// Demultiplies provided pixels alpha.
fn demultiply_alpha(data: &mut [RGBA8]) {
    for p in data {
        let a = p.a as f32 / 255.0;
        p.b = (p.b as f32 / a + 0.5) as u8;
        p.g = (p.g as f32 / a + 0.5) as u8;
        p.r = (p.r as f32 / a + 0.5) as u8;
    }
}

// TODO: https://github.com/rust-lang/rust/issues/44095
#[inline]
fn f32_bound(min: f32, val: f32, max: f32) -> f32 {
    debug_assert!(min.is_finite());
    debug_assert!(max.is_finite());

    if val > max {
        max
    } else if val >= min {
        val
    } else {
        // Catches `val < min` as well as a NaN `val`.
        min
    }
}

#[derive(Clone)]
struct Image {
    /// Filter primitive result.
    ///
    /// All images have the same size which is equal to the current filter region.
    image: Rc<tiny_skia::Pixmap>,

    /// Image's region that has actual data.
    ///
    /// Region is in global coordinates and not in `image` one.
    ///
    /// Image's content outside this region will be transparent/cleared.
    ///
    /// Currently used only for `feTile`.
    region: IntRect,

    /// The current color space.
    color_space: usvg::filter::ColorInterpolation,
}

impl Image {
    fn from_image(image: tiny_skia::Pixmap, color_space: usvg::filter::ColorInterpolation) -> Self {
        let (w, h) = (image.width(), image.height());
        Image {
            image: Rc::new(image),
            region: IntRect::from_xywh(0, 0, w, h).unwrap(),
            color_space,
        }
    }

    fn into_color_space(
        self,
        color_space: usvg::filter::ColorInterpolation,
    ) -> Result<Self, Error> {
        if color_space != self.color_space {
            let region = self.region;

            let mut image = self.take()?;

            match color_space {
                usvg::filter::ColorInterpolation::SRGB => image.into_srgb(),
                usvg::filter::ColorInterpolation::LinearRGB => image.into_linear_rgb(),
            }

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space,
            })
        } else {
            Ok(self)
        }
    }

    fn take(self) -> Result<tiny_skia::Pixmap, Error> {
        match Rc::try_unwrap(self.image) {
            Ok(v) => Ok(v),
            Err(v) => Ok((*v).clone()),
        }
    }

    fn width(&self) -> u32 {
        self.image.width()
    }

    fn height(&self) -> u32 {
        self.image.height()
    }

    fn as_ref(&self) -> &tiny_skia::Pixmap {
        &self.image
    }
}

struct FilterResult {
    name: String,
    image: Image,
}

pub fn apply(
    filter: &usvg::filter::Filter,
    ts: tiny_skia::Transform,
    source: &mut tiny_skia::Pixmap,
    ctx: &crate::render::Context,
) -> bool {
    let result = apply_inner(filter, ts, source, ctx);
    let result = result.and_then(|image| apply_to_canvas(image, source));

    // Clear on error.
    if result.is_err() {
        source.fill(tiny_skia::Color::TRANSPARENT);
    }

    let success = result.is_ok();
    match result {
        Ok(_) => {}
        Err(Error::InvalidRegion) => {
            log::warn!("Filter has an invalid region.");
        }
        Err(Error::NoResults) => {}
        Err(Error::WorkBudgetExceeded) => {}
        Err(Error::InvalidCoordinates) => {
            log::warn!("Filter coordinates exceed the integer domain.")
        }
    }
    success
}

fn apply_inner(
    filter: &usvg::filter::Filter,
    ts: usvg::Transform,
    source: &mut tiny_skia::Pixmap,
    ctx: &crate::render::Context,
) -> Result<Image, Error> {
    ctx.filter_work
        .reserve(Some(4096))
        .ok_or(Error::WorkBudgetExceeded)?;
    let region = filter
        .rect()
        .transform(ts)
        .and_then(|r| r.to_rect().round_out())
        .ok_or(Error::InvalidRegion)?;

    // The source pixmap is clamped to `max_bbox` in `render_group`, so the filter
    // region (derived directly from the unclamped filter rect) can be larger than
    // the buffer we actually render into.
    let source_rect =
        IntRect::from_xywh(0, 0, source.width(), source.height()).ok_or(Error::InvalidRegion)?;
    let region = crate::geom::fit_to_rect(region, source_rect).ok_or(Error::InvalidRegion)?;

    let mut results: Vec<FilterResult> = Vec::new();
    let last_uses = result_liveness::LastUses::new(filter.primitives());

    for (index, primitive) in filter.primitives().iter().enumerate() {
        // SourceGraphic clones the whole source; primitive subregions clip only
        // after execution. Reserve before input copies or algorithm allocations.
        let cost = work_budget::cost(primitive.kind(), source.width(), source.height(), ts);
        ctx.filter_work
            .reserve(cost)
            .ok_or(Error::WorkBudgetExceeded)?;
        let mut subregion = primitive
            .rect()
            .transform(ts)
            .and_then(|r| r.to_rect().round_out())
            .ok_or(Error::InvalidRegion)?;

        // `feOffset` inherits its region from the input.
        if let usvg::filter::Kind::Offset(fe) = primitive.kind() {
            if let usvg::filter::Input::Reference(name) = fe.input() {
                if let Some(res) = results.iter().rev().find(|v| v.name == *name) {
                    subregion = res.image.region;
                }
            }
        }

        let cs = primitive.color_interpolation();

        let mut result = match primitive.kind() {
            usvg::filter::Kind::Blend(fe) => {
                let input1 = get_input(fe.input1(), region, source, &results)?;
                let input2 = get_input(fe.input2(), region, source, &results)?;
                apply_blend(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::DropShadow(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_drop_shadow(fe, cs, ts, input)
            }
            usvg::filter::Kind::Flood(fe) => apply_flood(fe, region),
            usvg::filter::Kind::GaussianBlur(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_blur(fe, cs, ts, input)
            }
            usvg::filter::Kind::Offset(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_offset(fe, ts, input)
            }
            usvg::filter::Kind::Composite(fe) => {
                let input1 = get_input(fe.input1(), region, source, &results)?;
                let input2 = get_input(fe.input2(), region, source, &results)?;
                apply_composite(fe, cs, region, input1, input2)
            }
            usvg::filter::Kind::Merge(fe) => apply_merge(fe, cs, region, source, &results),
            usvg::filter::Kind::Tile(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_tile(input, region)
            }
            usvg::filter::Kind::Image(fe) => apply_image(fe, region, subregion, ts, ctx),
            usvg::filter::Kind::ComponentTransfer(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_component_transfer(fe, cs, input)
            }
            usvg::filter::Kind::ColorMatrix(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_color_matrix(fe, cs, input)
            }
            usvg::filter::Kind::ConvolveMatrix(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_convolve_matrix(fe, cs, input)
            }
            usvg::filter::Kind::Morphology(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_morphology(fe, cs, ts, input)
            }
            usvg::filter::Kind::DisplacementMap(fe) => {
                let input1 = get_input(fe.input1(), region, source, &results)?;
                let input2 = get_input(fe.input2(), region, source, &results)?;
                apply_displacement_map(fe, region, cs, ts, input1, input2)
            }
            usvg::filter::Kind::Turbulence(fe) => apply_turbulence(fe, region, cs, ts),
            usvg::filter::Kind::DiffuseLighting(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_diffuse_lighting(fe, region, cs, ts, input)
            }
            usvg::filter::Kind::SpecularLighting(fe) => {
                let input = get_input(fe.input(), region, source, &results)?;
                apply_specular_lighting(fe, region, cs, ts, input)
            }
        }?;

        if region != subregion {
            // Clip result.

            // TODO: explain
            let subregion2 = if let usvg::filter::Kind::Offset(..) = primitive.kind() {
                // We do not support clipping on feOffset.
                region.translate_to(0, 0)
            } else {
                subregion.translate(-region.x(), -region.y())
            }
            .unwrap();

            let color_space = result.color_space;

            let pixmap = {
                // This is cropping by clearing the pixels outside the region.
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(tiny_skia::Color::BLACK);
                paint.blend_mode = tiny_skia::BlendMode::Clear;

                let mut pixmap = result.take()?;
                let w = pixmap.width() as f32;
                let h = pixmap.height() as f32;

                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, w, subregion2.y() as f32) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, subregion2.x() as f32, h) {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) = tiny_skia::Rect::from_xywh(subregion2.right() as f32, 0.0, w, h)
                {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                if let Some(rect) =
                    tiny_skia::Rect::from_xywh(0.0, subregion2.bottom() as f32, w, h)
                {
                    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }

                pixmap
            };

            result = Image {
                image: Rc::new(pixmap),
                region: subregion,
                color_space,
            };
        }

        // Inputs have been consumed, including feOffset's region inheritance.
        // Preserve named branches until their last use, and always keep the new
        // result so the final primitive remains the filter output.
        results.retain(|result| last_uses.needed_after(&result.name, index));
        results.push(FilterResult {
            name: primitive.result().to_string(),
            image: result,
        });
    }

    if let Some(res) = results.pop() {
        Ok(res.image)
    } else {
        Err(Error::NoResults)
    }
}

fn get_input(
    input: &usvg::filter::Input,
    region: IntRect,
    source: &tiny_skia::Pixmap,
    results: &[FilterResult],
) -> Result<Image, Error> {
    match input {
        usvg::filter::Input::SourceGraphic => {
            let image = source.clone();

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space: usvg::filter::ColorInterpolation::SRGB,
            })
        }
        usvg::filter::Input::SourceAlpha => {
            let mut image = source.clone();
            // Set RGB to black. Keep alpha as is.
            for p in image.data_mut().as_rgba_mut() {
                p.r = 0;
                p.g = 0;
                p.b = 0;
            }

            Ok(Image {
                image: Rc::new(image),
                region,
                color_space: usvg::filter::ColorInterpolation::SRGB,
            })
        }
        usvg::filter::Input::Reference(name) => {
            if let Some(v) = results.iter().rev().find(|v| v.name == *name) {
                Ok(v.image.clone())
            } else {
                // Technically unreachable.
                log::warn!("Unknown filter primitive reference '{}'.", name);
                get_input(&usvg::filter::Input::SourceGraphic, region, source, results)
            }
        }
    }
}

trait PixmapToImageRef<'a> {
    fn as_image_ref(&'a self) -> ImageRef<'a>;
    fn as_image_ref_mut(&'a mut self) -> ImageRefMut<'a>;
}

impl<'a> PixmapToImageRef<'a> for tiny_skia::Pixmap {
    fn as_image_ref(&'a self) -> ImageRef<'a> {
        ImageRef::new(self.width(), self.height(), self.data().as_rgba())
    }

    fn as_image_ref_mut(&'a mut self) -> ImageRefMut<'a> {
        ImageRefMut::new(self.width(), self.height(), self.data_mut().as_rgba_mut())
    }
}

fn apply_drop_shadow(
    fe: &usvg::filter::DropShadow,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (dx, dy) = match scale_coordinates(fe.dx(), fe.dy(), ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    let mut pixmap = tiny_skia::Pixmap::try_create(input.width(), input.height())?;
    let input_pixmap = input.into_color_space(cs)?.take()?;
    let mut shadow_pixmap = input_pixmap.clone();

    if let Some((std_dx, std_dy)) = resolve_std_dev(fe.std_dev_x().get(), fe.std_dev_y().get(), ts)
    {
        gaussian_blur::apply(std_dx, std_dy, shadow_pixmap.as_image_ref_mut());
    }

    // flood
    let color = tiny_skia::Color::from_rgba8(
        fe.color().red,
        fe.color().green,
        fe.color().blue,
        fe.opacity().to_u8(),
    );
    for p in shadow_pixmap.pixels_mut() {
        let mut color = color;
        color.apply_opacity(p.alpha() as f32 / 255.0);
        *p = color.premultiply().to_color_u8();
    }

    match cs {
        usvg::filter::ColorInterpolation::SRGB => shadow_pixmap.into_srgb(),
        usvg::filter::ColorInterpolation::LinearRGB => shadow_pixmap.into_linear_rgb(),
    }

    pixmap.draw_pixmap(
        dx as i32,
        dy as i32,
        shadow_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    pixmap.draw_pixmap(
        0,
        0,
        input_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_blur(
    fe: &usvg::filter::GaussianBlur,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (std_dx, std_dy) = match resolve_std_dev(fe.std_dev_x().get(), fe.std_dev_y().get(), ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    let mut pixmap = input.into_color_space(cs)?.take()?;

    gaussian_blur::apply(std_dx, std_dy, pixmap.as_image_ref_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_offset(
    fe: &usvg::filter::Offset,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let (dx, dy) = match scale_coordinates(fe.dx(), fe.dy(), ts) {
        Some(v) => v,
        None => return Ok(input),
    };

    if dx.approx_zero_ulps(4) && dy.approx_zero_ulps(4) {
        return Ok(input);
    }

    let mut pixmap = tiny_skia::Pixmap::try_create(input.width(), input.height())?;
    // Keep the fractional translation for rasterization; do not truncate toward zero.
    pixmap.draw_pixmap(
        0,
        0,
        input.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::from_translate(dx, dy),
        None,
    );

    Ok(Image::from_image(pixmap, input.color_space))
}

fn apply_blend(
    fe: &usvg::filter::Blend,
    cs: usvg::filter::ColorInterpolation,
    region: IntRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let input1 = input1.into_color_space(cs)?;
    let input2 = input2.into_color_space(cs)?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    pixmap.draw_pixmap(
        0,
        0,
        input2.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    pixmap.draw_pixmap(
        0,
        0,
        input1.as_ref().as_ref(),
        &tiny_skia::PixmapPaint {
            blend_mode: crate::render::convert_blend_mode(fe.mode()),
            ..tiny_skia::PixmapPaint::default()
        },
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_composite(
    fe: &usvg::filter::Composite,
    cs: usvg::filter::ColorInterpolation,
    region: IntRect,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    use usvg::filter::CompositeOperator as Operator;

    if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator() {
        match composite::selected_input([k1, k2, k3, k4]) {
            Some(composite::SelectedInput::First) => return Ok(input1),
            Some(composite::SelectedInput::Second) => return Ok(input2),
            None => {}
        }
    }

    let input1 = input1.into_color_space(cs)?;
    let input2 = input2.into_color_space(cs)?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    if let Operator::Arithmetic { k1, k2, k3, k4 } = fe.operator() {
        let pixmap1 = input1.take()?;
        let pixmap2 = input2.take()?;

        composite::arithmetic(
            k1,
            k2,
            k3,
            k4,
            pixmap1.as_image_ref(),
            pixmap2.as_image_ref(),
            pixmap.as_image_ref_mut(),
        );

        return Ok(Image::from_image(pixmap, cs));
    }

    pixmap.draw_pixmap(
        0,
        0,
        input2.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    let blend_mode = match fe.operator() {
        Operator::Over => tiny_skia::BlendMode::SourceOver,
        Operator::In => tiny_skia::BlendMode::SourceIn,
        Operator::Out => tiny_skia::BlendMode::SourceOut,
        Operator::Atop => tiny_skia::BlendMode::SourceAtop,
        Operator::Xor => tiny_skia::BlendMode::Xor,
        Operator::Arithmetic { .. } => tiny_skia::BlendMode::SourceOver,
    };

    pixmap.draw_pixmap(
        0,
        0,
        input1.as_ref().as_ref(),
        &tiny_skia::PixmapPaint {
            blend_mode,
            ..tiny_skia::PixmapPaint::default()
        },
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_merge(
    fe: &usvg::filter::Merge,
    cs: usvg::filter::ColorInterpolation,
    region: IntRect,
    source: &tiny_skia::Pixmap,
    results: &[FilterResult],
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    for input in fe.inputs() {
        let input = get_input(input, region, source, results)?;
        let input = input.into_color_space(cs)?;
        pixmap.draw_pixmap(
            0,
            0,
            input.as_ref().as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
    }

    Ok(Image::from_image(pixmap, cs))
}

fn apply_flood(fe: &usvg::filter::Flood, region: IntRect) -> Result<Image, Error> {
    let c = fe.color();

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    // Keep opacity unquantized until premultiplication; quantizing alpha first
    // can perturb RGB even when the final alpha remains unchanged.
    pixmap.fill(
        tiny_skia::Color::from_rgba(
            f32::from(c.red) / 255.0,
            f32::from(c.green) / 255.0,
            f32::from(c.blue) / 255.0,
            fe.opacity().get(),
        )
        .ok_or(Error::InvalidRegion)?,
    );

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
}

fn apply_tile(input: Image, region: IntRect) -> Result<Image, Error> {
    let subregion = input.region.translate(-region.x(), -region.y()).unwrap();

    let tile_pixmap = input.image.copy_region(subregion)?;
    let mut paint = tiny_skia::Paint::default();
    paint.shader = tiny_skia::Pattern::new(
        tile_pixmap.as_ref(),
        tiny_skia::SpreadMode::Repeat,
        tiny_skia::FilterQuality::Bicubic,
        1.0,
        tiny_skia::Transform::from_translate(subregion.x() as f32, subregion.y() as f32),
    );

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;
    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, region.width() as f32, region.height() as f32)
        .unwrap();
    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
}

fn apply_image(
    fe: &usvg::filter::Image,
    region: IntRect,
    subregion: IntRect,
    ts: usvg::Transform,
    parent: &crate::render::Context,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = ts.get_scale();
    let transform = tiny_skia::Transform::from_row(
        sx,
        0.0,
        0.0,
        sy,
        subregion.x() as f32,
        subregion.y() as f32,
    );

    let ctx = crate::render::Context {
        filter_failed: parent.filter_failed,
        filter_work: parent.filter_work,
        max_bbox: tiny_skia::IntRect::from_xywh(0, 0, region.width(), region.height()).unwrap(),
    };

    crate::render::render_nodes(fe.root(), &ctx, transform, &mut pixmap.as_mut());
    if ctx.filter_failed.get() || ctx.filter_work.exhausted() {
        return Err(Error::InvalidCoordinates);
    }

    Ok(Image::from_image(
        pixmap,
        usvg::filter::ColorInterpolation::SRGB,
    ))
}

fn apply_component_transfer(
    fe: &usvg::filter::ComponentTransfer,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    if component_transfer::is_identity(fe) {
        return input.into_color_space(cs);
    }
    // Already-linear inputs retain their separately tested path.
    if input.color_space == usvg::filter::ColorInterpolation::SRGB {
        let mut pixmap = input.take()?;
        component_transfer_color::apply(
            fe,
            cs == usvg::filter::ColorInterpolation::LinearRGB,
            pixmap.data_mut(),
        );
        return Ok(Image::from_image(
            pixmap,
            usvg::filter::ColorInterpolation::SRGB,
        ));
    }
    let mut pixmap = input.into_color_space(cs)?.take()?;

    demultiply_alpha(pixmap.data_mut().as_rgba_mut());
    component_transfer::apply(fe, pixmap.as_image_ref_mut());
    multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_color_matrix(
    fe: &usvg::filter::ColorMatrix,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    if color_matrix::is_identity(fe.kind()) {
        return Ok(input);
    }
    // Pure channel selection commutes with componentwise color transforms.
    // Preserve the existing encoding and alpha instead of rounding a roundtrip.
    if let Some(mapping) = color_matrix::channel_mapping(fe.kind()) {
        let source_space = input.color_space;
        let mut pixmap = input.take()?;
        for pixel in pixmap.data_mut().as_rgba_mut() {
            let channels = [pixel.r, pixel.g, pixel.b];
            pixel.r = channels[mapping[0]];
            pixel.g = channels[mapping[1]];
            pixel.b = channels[mapping[2]];
        }
        return Ok(Image::from_image(pixmap, source_space));
    }
    if let usvg::filter::ColorMatrixKind::Matrix(matrix) = fe.kind() {
        let source_space = input.color_space;
        let transfer =
            (source_space != cs).then_some(cs == usvg::filter::ColorInterpolation::LinearRGB);
        let mut pixmap = input.take()?;
        matrix_premultiplied::apply(matrix, pixmap.as_image_ref_mut(), transfer);
        return Ok(Image::from_image(pixmap, source_space));
    }
    let mut pixmap = input.into_color_space(cs)?.take()?;

    demultiply_alpha(pixmap.data_mut().as_rgba_mut());
    color_matrix::apply(fe.kind(), pixmap.as_image_ref_mut());
    multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_convolve_matrix(
    fe: &usvg::filter::ConvolveMatrix,
    cs: usvg::filter::ColorInterpolation,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;

    if fe.preserve_alpha() {
        demultiply_alpha(pixmap.data_mut().as_rgba_mut());
    }

    convolve_matrix::apply(fe, pixmap.as_image_ref_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_morphology(
    fe: &usvg::filter::Morphology,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = input.into_color_space(cs)?.take()?;

    let (rx, ry) = match scale_coordinates(fe.radius_x().get(), fe.radius_y().get(), ts) {
        Some(v) => v,
        None => return Ok(Image::from_image(pixmap, cs)),
    };

    if rx == 0.0 && ry == 0.0 {
        return Ok(Image::from_image(pixmap, cs));
    }

    morphology::apply(fe.operator(), rx, ry, pixmap.as_image_ref_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_displacement_map(
    fe: &usvg::filter::DisplacementMap,
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input1: Image,
    input2: Image,
) -> Result<Image, Error> {
    let pixmap1 = input1.into_color_space(cs)?.take()?;
    let pixmap2 = input2.into_color_space(cs)?.take()?;

    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = match scale_coordinates(fe.scale(), fe.scale(), ts) {
        Some(v) => v,
        None => return Ok(Image::from_image(pixmap1, cs)),
    };

    displacement_map::apply(
        fe,
        sx,
        sy,
        pixmap1.as_image_ref(),
        pixmap2.as_image_ref(),
        pixmap.as_image_ref_mut(),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_turbulence(
    fe: &usvg::filter::Turbulence,
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let (sx, sy) = ts.get_scale();
    if sx.approx_zero_ulps(4) || sy.approx_zero_ulps(4) {
        return Ok(Image::from_image(pixmap, cs));
    }

    turbulence::apply(
        region.x() as f64 - ts.tx as f64,
        region.y() as f64 - ts.ty as f64,
        sx as f64,
        sy as f64,
        fe.base_frequency_x().get() as f64,
        fe.base_frequency_y().get() as f64,
        fe.num_octaves(),
        fe.seed(),
        fe.stitch_tiles(),
        fe.kind() == usvg::filter::TurbulenceKind::FractalNoise,
        pixmap.as_image_ref_mut(),
    )
    .ok_or(Error::InvalidCoordinates)?;

    multiply_alpha(pixmap.data_mut().as_rgba_mut());

    Ok(Image::from_image(pixmap, cs))
}

fn apply_diffuse_lighting(
    fe: &usvg::filter::DiffuseLighting,
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = transform_light_source(fe.light_source(), region, ts);

    lighting::diffuse_lighting(
        fe,
        light_source,
        input.as_ref().as_image_ref(),
        pixmap.as_image_ref_mut(),
    );

    Ok(Image::from_image(pixmap, cs))
}

fn apply_specular_lighting(
    fe: &usvg::filter::SpecularLighting,
    region: IntRect,
    cs: usvg::filter::ColorInterpolation,
    ts: usvg::Transform,
    input: Image,
) -> Result<Image, Error> {
    let mut pixmap = tiny_skia::Pixmap::try_create(region.width(), region.height())?;

    let light_source = transform_light_source(fe.light_source(), region, ts);

    lighting::specular_lighting(
        fe,
        light_source,
        input.as_ref().as_image_ref(),
        pixmap.as_image_ref_mut(),
    );

    Ok(Image::from_image(pixmap, cs))
}

// TODO: do not modify LightSource
fn transform_light_source(
    mut source: usvg::filter::LightSource,
    region: IntRect,
    ts: usvg::Transform,
) -> usvg::filter::LightSource {
    use std::f32::consts::SQRT_2;
    use usvg::filter::LightSource;

    match &mut source {
        LightSource::DistantLight(..) => {}
        LightSource::PointLight(light) => {
            let mut point = tiny_skia::Point::from_xy(light.x, light.y);
            ts.map_point(&mut point);
            light.x = point.x - region.x() as f32;
            light.y = point.y - region.y() as f32;
            light.z = light.z * (ts.sx * ts.sx + ts.sy * ts.sy).sqrt() / SQRT_2;
        }
        LightSource::SpotLight(light) => {
            let sz = (ts.sx * ts.sx + ts.sy * ts.sy).sqrt() / SQRT_2;

            let mut point = tiny_skia::Point::from_xy(light.x, light.y);
            ts.map_point(&mut point);
            light.x = point.x - region.x() as f32;
            light.y = point.y - region.y() as f32;
            light.z *= sz;

            let mut point = tiny_skia::Point::from_xy(light.points_at_x, light.points_at_y);
            ts.map_point(&mut point);
            light.points_at_x = point.x - region.x() as f32;
            light.points_at_y = point.y - region.y() as f32;
            light.points_at_z *= sz;
        }
    }

    source
}

fn apply_to_canvas(input: Image, pixmap: &mut tiny_skia::Pixmap) -> Result<(), Error> {
    let input = input.into_color_space(usvg::filter::ColorInterpolation::SRGB)?;

    pixmap.fill(tiny_skia::Color::TRANSPARENT);
    pixmap.draw_pixmap(
        0,
        0,
        input.as_ref().as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );

    Ok(())
}

/// Calculates Gaussian blur sigmas for the current world transform.
///
/// The blur implementation chooses a bounded kernel independently for each axis.
fn resolve_std_dev(std_dx: f32, std_dy: f32, ts: usvg::Transform) -> Option<(f64, f64)> {
    let (mut std_dx, mut std_dy) = scale_coordinates(std_dx, std_dy, ts)?;

    // 'A negative value or a value of zero disables the effect of the given filter primitive
    // (i.e., the result is the filter input image).'
    if std_dx.approx_eq_ulps(&0.0, 4) && std_dy.approx_eq_ulps(&0.0, 4) {
        return None;
    }

    // Preserve the existing tiny-sigma identity boundary.
    if std_dx < 0.05 {
        std_dx = 0.0;
    }

    if std_dy < 0.05 {
        std_dy = 0.0;
    }

    Some((std_dx as f64, std_dy as f64))
}

fn scale_coordinates(x: f32, y: f32, ts: usvg::Transform) -> Option<(f32, f32)> {
    let (sx, sy) = ts.get_scale();
    Some((x * sx, y * sy))
}
