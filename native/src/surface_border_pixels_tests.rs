//! 真实 WIC/D2D 四边像素、圆角、透明中心、塌缩与 alpha 接角，不以 source-string 当绘制证明。
use super::paint_pixels::{render, WIDTH};
fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * WIDTH + x) * 4..(y * WIDTH + x + 1) * 4]
}
#[test]
fn border_sides_pixels_draw_each_color_and_leave_center_transparent() {
    let pixels=render(br#"{"id":"root","type":"view","style":{"borderTopWidth":2,"borderRightWidth":4,"borderBottomWidth":6,"borderLeftWidth":8,"borderTopColor":"red","borderRightColor":"blue","borderBottomColor":"green","borderLeftColor":"yellow"}}"#);
    for (x, y, color) in [
        (32, 1, [0, 0, 255, 255]),
        (62, 32, [255, 0, 0, 255]),
        (32, 62, [0, 128, 0, 255]),
        (1, 32, [0, 255, 255, 255]),
        (32, 32, [0, 0, 0, 0]),
    ] {
        assert_eq!(pixel(&pixels, x, y), color, "pixel {x},{y}");
    }
}
#[test]
fn border_sides_pixels_share_uniform_alpha_mask_without_seams_or_double_blending() {
    for (width, radius) in [
        ([8; 4], 0),
        ([8; 4], 24),
        ([40; 4], 0),
        ([40; 4], 24),
        ([0, 4, 10, 20], 24),
        ([3, 0, 30, 7], 24),
    ] {
        let base = serde_json::json!({"id":"root","type":"view","style":{"borderTopWidth":width[0],"borderRightWidth":width[1],"borderBottomWidth":width[2],"borderLeftWidth":width[3],"borderRadius":radius,"borderColor":"rgba(255,0,0,.5)"}});
        let mut colored = base.clone();
        for (key, color) in [
            ("borderTopColor", "rgba(255,0,0,.5)"),
            ("borderRightColor", "rgba(0,0,255,.5)"),
            ("borderBottomColor", "rgba(0,255,0,.5)"),
            ("borderLeftColor", "rgba(255,255,0,.5)"),
        ] {
            colored["style"][key] = color.into();
        }
        let expected = render(&serde_json::to_vec(&base).unwrap());
        let actual = render(&serde_json::to_vec(&colored).unwrap());
        for (index, (a, b)) in actual
            .chunks_exact(4)
            .zip(expected.chunks_exact(4))
            .enumerate()
        {
            assert_eq!(
                a[3],
                b[3],
                "alpha at {},{} width={width:?} radius={radius}",
                index % WIDTH,
                index / WIDTH
            );
        }
    }
}
#[test]
fn border_sides_pixels_use_asymmetric_inner_corner_for_child_clipping() {
    let pixels=render(br#"{"id":"root","type":"view","layout":{"position":"relative","overflowX":"hidden","overflowY":"hidden"},"style":{"borderTopWidth":2,"borderRightWidth":6,"borderBottomWidth":12,"borderLeftWidth":18,"borderRadius":24,"borderColor":"red","background":"blue"},"children":[{"id":"child","type":"view","layout":{"position":"absolute","top":0,"right":0,"bottom":0,"left":0},"style":{"background":"green"}}]}"#);
    assert_eq!(pixel(&pixels, 1, 1), [0, 0, 0, 0]);
    assert_eq!(pixel(&pixels, 19, 3), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 25, 10), [0, 128, 0, 255]);
    assert_eq!(pixel(&pixels, 56, 50), [0, 0, 255, 255]);
}
