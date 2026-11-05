//! 不同于整图impulse用例，独立验证局部框、clip、非等比密度、栅格叠加与矢量隔离。
use super::*;

fn prepare(image: &Image, editor: &Editor, logical: Size) -> BlurImage {
    let snapshot = source_resolution::source_blur::prepare_active(
        image,
        editor,
        &source_resolution::Frame::from(logical),
        &AtomicBool::new(false),
    )
    .unwrap();
    snapshot.elements()[0].blur.clone().unwrap()
}

#[test]
fn projection_active_blur_nonuniform_density_preserves_clip_mask_and_gaussian_axes() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 8 * 8 * 4].into(),
    };
    let logical = Size {
        width: 4.0,
        height: 2.0,
    };
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 4.0,
        height: 2.0,
    };
    let clip = Rect {
        x: 1.0,
        y: 0.5,
        width: 2.0,
        height: 1.0,
    };
    let blur = BlurImage::new(1, 1, vec![0; 4].into(), 1)
        .unwrap()
        .with_frame(frame)
        .unwrap()
        .with_effect_clip(clip)
        .unwrap()
        .with_effect_mask_scale((0.75, 0.5))
        .unwrap();
    let mut editor = Editor::default();
    editor.begin_blur(Point { x: 2.0, y: 1.0 }, blur).unwrap();
    editor.commit();
    let prepared = prepare(&image, &editor, logical);
    assert_eq!((prepared.width(), prepared.height()), (8, 8));
    assert_eq!(prepared.frame(), frame);
    assert_eq!(prepared.effect_clip(), Some(clip));
    assert_eq!(prepared.effect_mask_scale(), (0.75, 0.5));
    // contain+clip精确对应输出[2,6)×[2,6)白色，其余透明；独立f64可分离卷积。
    for y in 0..8usize {
        for x in 0..8usize {
            let axis =
                |p: usize, sigma: f64| (2..6).map(|q| weight(p as i32 - q, sigma)).sum::<f64>();
            let expected = (255.0 * axis(x, 2.0) * axis(y, 4.0)).round() as u8;
            for channel in 0..4 {
                let actual = prepared.pixels()[(y * 8 + x) * 4 + channel];
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "{x},{y},{channel}: {actual} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn projection_active_blur_local_sampling_composes_raster_but_never_recurses_into_vectors() {
    let image = Image {
        width: 16,
        height: 8,
        pixels: [80, 0, 0, 255].repeat(16 * 8).into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 4),
        base: vec![0; 8 * 4 * 4].into(),
        annotations: Some([0, 40, 0, 128].repeat(8 * 4).into()),
        annotation_size: None,
    })
    .unwrap();
    let frame = Rect {
        x: 2.0,
        y: 1.0,
        width: 4.0,
        height: 2.0,
    };
    editor
        .begin_blur(
            Point { x: 3.0, y: 2.0 },
            BlurImage::with_strength(1, 1, vec![0; 4].into(), 1.25)
                .unwrap()
                .with_frame(frame)
                .unwrap(),
        )
        .unwrap();
    editor.commit();
    editor.set_style(
        crate::stroke_style::StrokeStyle::new(8.0, crate::stroke_style::Color(255, 0, 0)).unwrap(),
    );
    editor
        .begin(crate::editor::Tool::Brush, Point { x: 4.0, y: 2.0 })
        .unwrap();
    editor.commit();
    let stamp = editor.document_stamp();
    let blur = prepare(
        &image,
        &editor,
        Size {
            width: 8.0,
            height: 4.0,
        },
    );
    assert_eq!((blur.width(), blur.height()), (8, 4));
    assert_eq!(blur.frame(), frame);
    assert_eq!(blur.strength(), 1.25);
    // 原卷积器在纹理外使用透明，不做边界renormalize；局部框边缘不能期望常量不变。
    for y in 0..4usize {
        for x in 0..8usize {
            let horizontal = (0..8).map(|i| weight(x as i32 - i, 2.5)).sum::<f64>();
            let vertical = (0..4).map(|i| weight(y as i32 - i, 2.5)).sum::<f64>();
            let pixel = &blur.pixels()[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4];
            for (actual, base) in pixel.iter().zip([40u8, 40, 0, 255]) {
                let expected = (base as f64 * horizontal * vertical).round() as u8;
                assert!(actual.abs_diff(expected) <= 1, "{x},{y}: {pixel:?}");
            }
        }
    }
    assert!(stamp.matches(&editor));
    assert_eq!(editor.elements().len(), 2);
}

#[test]
fn projection_active_blur_clips_raster_colors_before_convolution_not_only_the_base() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![0; 8 * 8 * 4].into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (4, 2),
        base: vec![0; 4 * 2 * 4].into(),
        annotations: Some(vec![255; 4 * 2 * 4].into()),
        annotation_size: None,
    })
    .unwrap();
    let blur = BlurImage::new(1, 1, vec![0; 4].into(), 1)
        .unwrap()
        .with_frame(Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 2.0,
        })
        .unwrap()
        .with_effect_clip(Rect {
            x: 1.0,
            y: 0.5,
            width: 2.0,
            height: 1.0,
        })
        .unwrap();
    editor.begin_blur(Point { x: 2.0, y: 1.0 }, blur).unwrap();
    editor.commit();
    let result = prepare(
        &image,
        &editor,
        Size {
            width: 4.0,
            height: 2.0,
        },
    );
    for y in 0..8usize {
        for x in 0..8usize {
            let axis =
                |p: usize, sigma: f64| (2..6).map(|q| weight(p as i32 - q, sigma)).sum::<f64>();
            let expected = (255.0 * axis(x, 2.0) * axis(y, 4.0)).round() as u8;
            assert!(
                result.pixels()[(y * 8 + x) * 4].abs_diff(expected) <= 1,
                "{x},{y}"
            );
        }
    }
}
