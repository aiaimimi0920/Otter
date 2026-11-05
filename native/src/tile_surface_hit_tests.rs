//! 纯数值独立oracle：像素中心、四向旋转、crop及遮挡/半开边界。
use super::*;
#[test]
fn rotated_pixel_centers_and_crop_match_independent_corner_oracle() {
    use crate::tile_wire::Rotation::*;
    let f = crate::tile_surface_actions::tests::Fixture::new("art");
    for (rotation, expected) in [
        (Zero, [0.25, 0.25]),
        (Right, [0.75, 0.25]),
        (Half, [0.75, 0.75]),
        (Left, [0.25, 0.75]),
    ] {
        let mut l = f.job.layout.clone();
        l.tiles[0].rotation = rotation;
        assert_eq!(
            hit(&l, &f.job.key.endpoint, f.job.size, [0, 0]).unwrap().1,
            expected
        );
        l.placements[0].source_crop = crate::tile_wire::Rect {
            x: 0.25,
            y: 0.1,
            width: 0.5,
            height: 0.8,
        };
        let uv = hit(&l, &f.job.key.endpoint, f.job.size, [0, 0]).unwrap().1;
        assert!((uv[0] - (0.25 + expected[0] * 0.5)).abs() < 1e-6);
        assert!((uv[1] - (0.1 + expected[1] * 0.8)).abs() < 1e-6);
    }
}
#[test]
fn top_noninteractive_or_image_occludes_art_and_tie_order_is_stable() {
    let f = crate::tile_surface_actions::tests::Fixture::new("art");
    let mut l = f.job.layout.clone();
    let mut top = l.placements[0].clone();
    top.placement_id = "z-top".into();
    top.interactive = false;
    l.placements.push(top);
    assert!(hit(&l, &f.job.key.endpoint, f.job.size, [0, 0]).is_none());
    l.placements[1].interactive = true;
    l.placements[1].source.kind = "image".into();
    assert!(hit(&l, &f.job.key.endpoint, f.job.size, [0, 0]).is_none());
    l.placements[1].source.kind = "surface".into();
    assert_eq!(
        hit(&l, &f.job.key.endpoint, f.job.size, [0, 0])
            .unwrap()
            .0
            .placement_id,
        "z-top"
    );
    l.placements.swap(0, 1);
    assert_eq!(
        hit(&l, &f.job.key.endpoint, f.job.size, [0, 0])
            .unwrap()
            .0
            .placement_id,
        "z-top"
    );
    for pixel in [[-1, 0], [0, -1], [2, 0], [0, 2]] {
        assert!(hit(&l, &f.job.key.endpoint, f.job.size, pixel).is_none());
    }
    l.placements.iter_mut().for_each(|p| p.rect.width = 0.5);
    assert!(
        hit(&l, &f.job.key.endpoint, f.job.size, [0, 0]).is_none(),
        "x=0.5 is the exclusive right edge"
    );
}
