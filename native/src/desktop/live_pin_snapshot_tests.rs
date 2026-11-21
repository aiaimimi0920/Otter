//! 当前采集底图刷新不能丢掉已有标注、镜像及模糊，也不能按字节数误认尺寸。
use super::*;
use crate::{editor::Tool, viewport::Point};

#[test]
fn live_pin_refresh_resizes_and_mirrors_new_pixels_and_preserves_overlay() {
    let overlay = [3, 0, 0, 3].repeat(4).into();
    let mut editor = Editor::with_layers(Layers {
        size: (2, 2),
        base: [0, 0, 0, 255].repeat(4).into(),
        annotations: Some(overlay),
        annotation_size: None,
    })
    .unwrap();
    editor.begin(Tool::Brush, Point { x: 0.5, y: 0.5 }).unwrap();
    assert!(editor.commit());
    let plan = editor
        .prepare_flip(crate::raster_flip::Axis::Horizontal, |_| unreachable!())
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    assert!(editor.commit_raster_refresh(plan).unwrap());
    let annotations = editor.layers().unwrap().annotations.clone().unwrap();
    let image = Image {
        width: 4,
        height: 1,
        pixels: [10u8, 30, 50, 70]
            .into_iter()
            .flat_map(|v| [v, 0, 0, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let (next, image) = refresh(editor.export_snapshot(), image, || false).unwrap();
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(&image.pixels[..], &[60, 0, 0, 255, 20, 0, 0, 255].repeat(2));
    assert!(next
        .layers()
        .unwrap()
        .annotations
        .as_ref()
        .unwrap()
        .same_storage(&annotations));
    assert_eq!(next.elements().len(), 1);
    assert!(next.mirror_state().x);
    assert_eq!(editor.layers().unwrap().base[0], 0);
}

#[test]
fn live_pin_refresh_recomputes_blur_and_adoption_preserves_undo() {
    let mut editor = Editor::with_layers(Layers {
        size: (16, 16),
        base: [0, 0, 255, 255].repeat(256).into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    let blur = crate::blur_image::BlurImage::with_strength(
        16,
        16,
        [0, 0, 255, 255].repeat(256).into(),
        2.4,
    )
    .unwrap();
    editor.begin_blur(Point { x: 8.0, y: 8.0 }, blur).unwrap();
    assert!(editor.commit());
    let image = Image {
        width: 16,
        height: 16,
        pixels: [0, 255, 0, 255].repeat(256).into(),
    };
    let (next, _) = refresh(editor.export_snapshot(), image, || false).unwrap();
    let expected = crate::blur::region_precise(
        &[0, 255, 0, 255].repeat(256).into(),
        (16, 16),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 16,
            height: 16,
        },
        2.4,
        || false,
    )
    .unwrap();
    assert_eq!(
        &next.elements()[0].blur.as_ref().unwrap().pixels()[..],
        &expected[..]
    );
    assert!(editor.adopt_snapshot(editor.revision(), &next).unwrap());
    assert_eq!(editor.layers().unwrap().base[1], 255);
    assert!(editor.undo());
    assert_eq!(editor.layers().unwrap().base[2], 255);
    assert!(editor.undo());
    assert!(editor.elements().is_empty());
}
