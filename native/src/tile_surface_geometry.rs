//! canonical字段裁剪到source crop后映射到输出；四向旋转只由tile_geometry处理一次。
use crate::{
    surface_clip::Rect, tile_shared::Frame, tile_surface_owner::Art, tile_wire::Placement,
};
pub(crate) fn viewport(frame: &Frame, art: &Art, p: &Placement) -> Option<Rect> {
    let tile = frame
        .layout
        .tiles
        .iter()
        .find(|t| t.endpoint_id == frame.key.endpoint)?;
    let low = [p.rect.x.max(tile.rect.x), p.rect.y.max(tile.rect.y)];
    let high = [
        (p.rect.x + p.rect.width).min(tile.rect.x + tile.rect.width),
        (p.rect.y + p.rect.height).min(tile.rect.y + tile.rect.height),
    ];
    if low[0] >= high[0] || low[1] >= high[1] {
        return None;
    }
    let a = crate::tile_surface_hit::source(p, low);
    let b = crate::tile_surface_hit::source(p, high);
    let r = Rect {
        left: a[0] * art.size[0] as f32,
        top: a[1] * art.size[1] as f32,
        right: b[0] * art.size[0] as f32,
        bottom: b[1] * art.size[1] as f32,
    };
    (!r.empty()).then_some(r)
}
pub(crate) fn map(frame: &Frame, art: &Art, p: &Placement, point: [f32; 2]) -> Option<[f64; 2]> {
    let tile = frame
        .layout
        .tiles
        .iter()
        .find(|t| t.endpoint_id == frame.key.endpoint)?;
    let wall = [
        p.rect.x
            + (f64::from(point[0]) / f64::from(art.size[0]) - p.source_crop.x)
                / p.source_crop.width
                * p.rect.width,
        p.rect.y
            + (f64::from(point[1]) / f64::from(art.size[1]) - p.source_crop.y)
                / p.source_crop.height
                * p.rect.height,
    ];
    let native = crate::tile_geometry::native(tile, wall);
    Some([
        native[0] * f64::from(frame.size.width),
        native[1] * f64::from(frame.size.height),
    ])
}
pub(crate) fn bounds(frame: &Frame, art: &Art, p: &Placement, bounds: Rect) -> Option<[f64; 4]> {
    let c = p.source_crop;
    let crop = Rect {
        left: c.x as f32 * art.size[0] as f32,
        top: c.y as f32 * art.size[1] as f32,
        right: (c.x + c.width) as f32 * art.size[0] as f32,
        bottom: (c.y + c.height) as f32 * art.size[1] as f32,
    };
    let r = bounds.intersect(crop);
    if r.empty() {
        return None;
    }
    let corners = [
        [r.left, r.top],
        [r.right, r.top],
        [r.left, r.bottom],
        [r.right, r.bottom],
    ]
    .map(|v| map(frame, art, p, v));
    let [Some(a), Some(b), Some(c), Some(d)] = corners else {
        return None;
    };
    let corners = [a, b, c, d];
    let rect = [
        corners
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min)
            .max(0.),
        corners
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min)
            .max(0.),
        corners
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .min(f64::from(frame.size.width)),
        corners
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .min(f64::from(frame.size.height)),
    ];
    (rect.iter().all(|v| v.is_finite()) && rect[2] > rect[0] && rect[3] > rect[1]).then_some(rect)
}
