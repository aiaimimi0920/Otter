//! 搜索区域的保守快速剔除；只有凸形覆盖证据才整块跳过，圆角外侧保留精确搜索。
use super::*;
pub(super) fn covered(
    frame: &Frame,
    art: &Arc<Art>,
    p: &Presentation,
    placement: &Placement,
    node: &str,
    rect: [i32; 4],
) -> bool {
    let [l, t, r, b] = rect;
    let pixels = [[l, t], [r - 1, t], [l, b - 1], [r - 1, b - 1]];
    let walls = pixels
        .map(|v| crate::tile_surface_hit::point(&frame.layout, &frame.key.endpoint, frame.size, v));
    if frame.layout.placements.iter().any(|other| {
        (other.z_index, &other.placement_id) > (placement.z_index, &placement.placement_id)
            && walls.iter().all(|v| {
                v.is_some_and(|v| {
                    v[0] >= other.rect.x
                        && v[0] < other.rect.x + other.rect.width
                        && v[1] >= other.rect.y
                        && v[1] < other.rect.y + other.rect.height
                })
            })
    }) {
        return true;
    }
    let Some(index) = p.geometry().nodes().iter().position(|g| g.id() == node) else {
        return false;
    };
    let hits = pixels.map(|pixel| {
        let (hit, uv) =
            crate::tile_surface_hit::hit(&frame.layout, &frame.key.endpoint, frame.size, pixel)?;
        if hit.placement_id != placement.placement_id {
            return None;
        }
        p.hit_test([uv[0] * art.size[0] as f32, uv[1] * art.size[1] as f32])
    });
    // 单节点自身和祖先圆角都是凸形。其四个像素中心均覆盖时，整个像素矩形也被覆盖。
    hits[0].is_some_and(|id| {
        hits.iter().all(|hit| *hit == Some(id))
            && p.geometry().nodes()[index + 1..]
                .iter()
                .any(|g| g.id() == id)
    })
}
