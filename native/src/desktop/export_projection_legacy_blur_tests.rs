//! 当前legacy二次裁剪仍需采完整邻域；期望由矩形离散高斯积分独立产生。
use super::*;

#[test]
fn projection_active_blur_explicit_nested_legacy_keeps_full_neighborhood_and_empty_region() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 256].into(),
    };
    for (x, width, expected) in [(2.0, 2.0, 0..8), (-4.0, 8.0, 4..8), (100.0, 8.0, 0..0)] {
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (8, 8),
            base: vec![0; 256].into(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_size(logical())
        .unwrap()
        .with_restored_legacy_crop(
            Rect {
                x,
                y: 0.0,
                width,
                height: 8.0,
            },
            logical(),
        )
        .unwrap();
        let blur = BlurImage::new(8, 8, vec![0; 256].into(), 1).unwrap();
        editor.begin_blur(Point { x: 4.0, y: 4.0 }, blur).unwrap();
        assert!(editor.commit());
        editor
            .crop_to(Rect {
                x: 2.0,
                y: 2.0,
                width: 4.0,
                height: 4.0,
            })
            .unwrap();
        for pending in [false, true] {
            let source = record(Value::Null, Value::Null);
            let target = target::read(&source, "target").unwrap();
            let frame = target
                .content(
                    &image,
                    &editor,
                    logical().into(),
                    logical(),
                    &Input {
                        legacy_pixels: editor.crop_view().unwrap().legacy_pixel_region(),
                        content: pending,
                        ..Input::default()
                    },
                    None,
                )
                .unwrap();
            let prepared = source_resolution::source_blur::prepare_active(
                &image,
                &editor,
                &frame,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_rectangle(
                prepared.elements()[0].blur.as_ref().unwrap(),
                expected.clone(),
                0..8,
            );
        }
    }
}
