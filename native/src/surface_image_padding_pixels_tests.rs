//! 非对称 padding 的真实 content-edge 图片与四角裁剪，复用原 WIC/Presentation 夹具。
use super::*;
fn solid() -> Rc<Bitmap> {
    Bitmap::new(
        Image::from_surface(
            &[255, 0, 0, 255],
            "application/x-neuro-rgba8",
            Some((1, 1)),
            || false,
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn padding_image_asymmetric_content_origin_and_contain_pixels() {
    let p=render_source(br#"{"id":"image","type":"image","layout":{"width":64,"height":64,"paddingLeft":10,"paddingRight":30,"paddingTop":5,"paddingBottom":15}}"#,solid());
    // 内容 24x44，1:1 图片居中为 24x24，不能按 left 或 top 做对称内缩。
    assert_eq!(pixel(&p, 10, 15), [0, 0, 255, 255]);
    assert_eq!(pixel(&p, 33, 38), [0, 0, 255, 255]);
    for (x, y) in [(9, 20), (34, 20), (20, 14), (20, 39)] {
        assert_eq!(pixel(&p, x, y), [0, 0, 0, 0]);
    }
}
#[test]
fn padding_image_content_corners_subtract_each_adjacent_side_independently() {
    // 图片比例恰好填满 50x46 内容区，不能让 contain 留白掩盖角上的错误。
    let json=br#"{"id":"image","type":"image","layout":{"width":64,"height":64,"paddingLeft":2,"paddingRight":12,"paddingTop":4,"paddingBottom":14},"style":{"borderRadius":20}}"#;
    let source = Bitmap::new(
        Image::from_surface(
            &[255, 0, 0, 255].repeat(50 * 46),
            "application/x-neuro-rgba8",
            Some((50, 46)),
            || false,
        )
        .unwrap(),
    )
    .unwrap();
    let p = render_source(json, source);
    for (x, y) in [(3, 5), (50, 5), (3, 48), (50, 48)] {
        assert!(pixel(&p, x, y)[3] < 16, "corner {x},{y}");
    }
    for (x, y) in [(20, 5), (50, 20), (3, 20), (20, 48)] {
        assert!(pixel(&p, x, y)[3] > 240, "edge {x},{y}");
    }
    for (x, y) in [(1, 20), (52, 20), (20, 3), (20, 50)] {
        assert_eq!(pixel(&p, x, y), [0, 0, 0, 0]);
    }
}
