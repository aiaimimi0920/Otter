//! SVG 图片自然尺寸与内部 viewport；不从绘制内容包围盒猜测尺寸。
use super::invalid;
use resvg::usvg::roxmltree::Node;
use std::io;
use svgtypes::{Length, LengthUnit, ViewBox};

pub(super) struct Viewport {
    pub logical_size: [f32; 2],
    pub width: u32,
    pub height: u32,
    pub inner_width: f64,
    pub inner_height: f64,
}
fn length(value: Option<&str>) -> io::Result<Option<Length>> {
    value
        .map(|v| v.parse::<Length>().map_err(|_| invalid()))
        .transpose()
}
fn absolute(value: Option<Length>) -> io::Result<Option<f64>> {
    let Some(value) = value else { return Ok(None) };
    if !value.number.is_finite() || value.number <= 0.0 {
        return Err(invalid());
    }
    let factor = match value.unit {
        LengthUnit::None | LengthUnit::Px => 1.0,
        LengthUnit::In => 96.0,
        LengthUnit::Cm => 96.0 / 2.54,
        LengthUnit::Mm => 96.0 / 25.4,
        LengthUnit::Pt => 96.0 / 72.0,
        LengthUnit::Pc => 16.0,
        LengthUnit::Percent => return Ok(None),
        _ => return Err(invalid()), // 字体相对单位尚未建立字体合同。
    };
    Ok(Some(value.number * factor))
}
pub(super) fn inspect(root: Node<'_, '_>) -> io::Result<Viewport> {
    let w = length(root.attribute("width"))?;
    let h = length(root.attribute("height"))?;
    let ratio = root
        .attribute("viewBox")
        .map(|v| {
            let v = v.parse::<ViewBox>().map_err(|_| invalid())?;
            if ![v.x, v.y, v.w, v.h].iter().all(|n| n.is_finite()) || v.w <= 0.0 || v.h <= 0.0 {
                return Err(invalid());
            }
            Ok(v.w / v.h)
        })
        .transpose()?;
    let (width, height) = match (absolute(w)?, absolute(h)?, ratio) {
        (Some(w), Some(h), _) => (w, h),
        (Some(w), None, Some(r)) => (w, w / r),
        (None, Some(h), Some(r)) => (h * r, h),
        (Some(w), None, None) => (w, 150.0),
        (None, Some(h), None) => (300.0, h),
        (None, None, Some(r)) if r > 2.0 => (300.0, 300.0 / r),
        (None, None, Some(r)) => (150.0 * r, 150.0),
        (None, None, None) => (300.0, 150.0),
    };
    // 像素 extent 向上取整，但逻辑几何保持小数，不能用像素取整结果放大内容。
    if [width, height]
        .iter()
        .any(|n| !n.is_finite() || *n <= 0.0 || *n > 16384.0)
    {
        return Err(invalid());
    }
    let logical_size = [width as f32, height as f32];
    let raster = logical_size.map(|v| v.ceil() as u32);
    crate::surface_image_worker_wire::logical_size(logical_size, raster)?;
    crate::surface_image_worker_wire::pixel_count(raster[0], raster[1])?;
    let inner = |value: Option<Length>, size: f64| match value {
        Some(v) if v.unit == LengthUnit::Percent => size * v.number / 100.0,
        _ => size,
    };
    let (inner_width, inner_height) = (inner(w, width), inner(h, height));
    if [inner_width, inner_height]
        .iter()
        .any(|n| !n.is_finite() || *n <= 0.0 || *n > 16384.0)
    {
        return Err(invalid());
    }
    Ok(Viewport {
        logical_size,
        width: raster[0],
        height: raster[1],
        inner_width,
        inner_height,
    })
}
