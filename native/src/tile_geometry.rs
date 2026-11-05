//! 与 Web Wall 几何一致：物理像素中心→墙坐标，图片 crop 与 Tile 旋转只应用一次。
use crate::tile_wire::{Placement, Rect, Rotation, Tile};
pub(crate) fn rotate([u, v]: [f64; 2], rotation: Rotation) -> [f64; 2] {
    match rotation {
        Rotation::Zero => [u, v],
        Rotation::Right => [1. - v, u],
        Rotation::Half => [1. - u, 1. - v],
        Rotation::Left => [v, 1. - u],
    }
}
pub(crate) fn native(tile: &Tile, [x, y]: [f64; 2]) -> [f64; 2] {
    let inverse = match tile.rotation {
        Rotation::Right => Rotation::Left,
        Rotation::Left => Rotation::Right,
        r => r,
    };
    rotate(
        [
            (x - tile.rect.x) / tile.rect.width,
            (y - tile.rect.y) / tile.rect.height,
        ],
        inverse,
    )
}
pub(crate) struct Projection {
    pub quad: [[f64; 2]; 3],
    pub crop: Rect,
}
pub(crate) fn project(tile: &Tile, p: &Placement) -> Option<Projection> {
    let (x, y) = (tile.rect.x.max(p.rect.x), tile.rect.y.max(p.rect.y));
    let (right, bottom) = (
        (tile.rect.x + tile.rect.width).min(p.rect.x + p.rect.width),
        (tile.rect.y + tile.rect.height).min(p.rect.y + p.rect.height),
    );
    if x >= right || y >= bottom {
        return None;
    }
    Some(Projection {
        quad: [
            native(tile, [x, y]),
            native(tile, [right, y]),
            native(tile, [x, bottom]),
        ],
        crop: Rect {
            x: p.source_crop.x + (x - p.rect.x) / p.rect.width * p.source_crop.width,
            y: p.source_crop.y + (y - p.rect.y) / p.rect.height * p.source_crop.height,
            width: (right - x) / p.rect.width * p.source_crop.width,
            height: (bottom - y) / p.rect.height * p.source_crop.height,
        },
    })
}
