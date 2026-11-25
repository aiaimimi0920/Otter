//! Physical pixel centers to topmost Wall placements and canonical Art coordinates; no occlusion pass-through, with four-way rotation applied once.
use crate::{
    tile_geometry::rotate,
    tile_wire::{Layout, Placement, Size},
};
pub(crate) fn hit<'a>(
    layout: &'a Layout,
    endpoint: &str,
    size: Size,
    pixel: [i32; 2],
) -> Option<(&'a Placement, [f32; 2])> {
    if pixel[0] < 0
        || pixel[1] < 0
        || pixel[0] as u32 >= size.width
        || pixel[1] as u32 >= size.height
    {
        return None;
    }
    let point = point(layout, endpoint, size, pixel)?;
    let p = layout
        .placements
        .iter()
        .filter(|p| {
            point[0] >= p.rect.x
                && point[1] >= p.rect.y
                && point[0] < p.rect.x + p.rect.width
                && point[1] < p.rect.y + p.rect.height
        })
        .max_by(|a, b| {
            a.z_index
                .cmp(&b.z_index)
                .then(a.placement_id.cmp(&b.placement_id))
        })?;
    if !p.interactive || p.source.kind != "surface" {
        return None;
    }
    Some((p, source(p, point)))
}
/// Captured dragging may leave controls; transform coordinates only without granting hit/action permissions.
pub(crate) fn point(
    layout: &Layout,
    endpoint: &str,
    size: Size,
    pixel: [i32; 2],
) -> Option<[f64; 2]> {
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let tile = layout.tiles.iter().find(|t| t.endpoint_id == endpoint)?;
    let uv = rotate(
        [
            (f64::from(pixel[0]) + 0.5) / f64::from(size.width),
            (f64::from(pixel[1]) + 0.5) / f64::from(size.height),
        ],
        tile.rotation,
    );
    Some([
        tile.rect.x + uv[0] * tile.rect.width,
        tile.rect.y + uv[1] * tile.rect.height,
    ])
}
pub(crate) fn source(p: &Placement, point: [f64; 2]) -> [f32; 2] {
    [
        (p.source_crop.x + (point[0] - p.rect.x) / p.rect.width * p.source_crop.width) as f32,
        (p.source_crop.y + (point[1] - p.rect.y) / p.rect.height * p.source_crop.height) as f32,
    ]
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_surface_hit_tests.rs"]
mod tests;
