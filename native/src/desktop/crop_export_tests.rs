//! 真实 WIC 编码/解码验证裁剪边界与各图层一致，不写入用户文件。
use super::*;
use crate::{crop_geometry::Rect, editor::Tool, erase_layers::Layers, stroke_style::Color};

fn document() -> (Image, Editor) {
    let image = Image {
        width: 32,
        height: 24,
        pixels: (0..32 * 24)
            .flat_map(|i| [(i % 32 * 4) as u8, (i / 32 * 6) as u8, 30, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size: (32, 24),
        base: image.pixels.clone(),
        annotations: None,
    })
    .unwrap();
    editor.set_style(
        editor
            .style()
            .with_color(Color(220, 10, 40))
            .with_width(3.0)
            .unwrap(),
    );
    editor
        .begin(Tool::Rectangle, Point { x: 9.0, y: 6.0 })
        .unwrap();
    editor.extend(Point { x: 21.0, y: 16.0 }).unwrap();
    editor.commit();
    editor.rotate(0, 25.0).unwrap();
    (image, editor)
}

#[test]
fn reset_fractional_restored_frame_exports_logical_dimensions_not_buffer_padding() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 4,
        pixels: vec![0; 128].into(),
    };
    let mut editor = Editor::with_layers(Layers {
        size: (8, 4),
        base: image.pixels.clone(),
        annotation_size: Some((1, 1)),
        annotations: Some(vec![0, 0, 255, 255].into()),
    })
    .unwrap()
    .with_restored_crop(crate::crop_geometry::Crop {
        rect: Rect {
            x: 1.0,
            y: 0.5,
            width: 2.25,
            height: 1.25,
        },
        source: crate::crop_geometry::Size {
            width: 7.25,
            height: 3.25,
        },
    })
    .unwrap();
    let cropped = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!((cropped.width, cropped.height), (2, 1));
    assert!(editor.reset_crop());
    let full = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!((full.width, full.height), (7, 3));
    assert!(full
        .pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 0, 255, 255]));
    assert!(editor.undo());
    let restored = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&restored.pixels[..], &cropped.pixels[..]);
}

#[test]
fn independent_annotation_dimensions_export_at_frame_size_and_survive_history() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![0; 256].into(),
    };
    let source = Layers {
        annotation_size: Some((2, 1)),
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some(vec![0, 0, 255, 255, 0, 255, 0, 255].into()),
    };
    let mut editor = Editor::with_layers(source.clone()).unwrap();
    let full = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!((full.width, full.height), (8, 8));
    assert_eq!(&full.pixels[0..4], &[0, 0, 255, 255]);
    assert_eq!(&full.pixels[28..32], &[0, 255, 0, 255]);
    assert_eq!(&full.pixels[224..228], &[0, 0, 255, 255]);
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 3.0,
            width: 4.0,
            height: 2.0,
        })
        .unwrap();
    let cropped = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!((cropped.width, cropped.height), (4, 2));
    assert_eq!(&cropped.pixels[0..4], &[0, 0, 255, 255]);
    assert_eq!(&cropped.pixels[12..16], &[0, 255, 0, 255]);
    let mut vertical = source.clone();
    vertical.annotation_size = Some((1, 2));
    let plan = editor
        .prepare_raster_refresh(vertical)
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    assert!(editor.commit_raster_refresh(plan).unwrap());
    assert_eq!(editor.layers().unwrap().annotation_dimensions(), (1, 2));
    assert!(editor.undo());
    assert!(editor.layers().unwrap().same_storage(&source));
    let restored = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&restored.pixels[..], &cropped.pixels[..]);
    assert!(editor.redo());
    assert_eq!(editor.layers().unwrap().annotation_dimensions(), (1, 2));
}

#[test]
fn cropped_raster_annotation_fits_visible_frame_instead_of_clipping_source() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 8,
        pixels: [0, 0, 0, 255].repeat(64).into(),
    };
    let overlay = (0..64)
        .flat_map(|i| {
            if i % 8 < 4 {
                [0, 0, 200, 255]
            } else {
                [0, 200, 0, 255]
            }
        })
        .collect::<Vec<_>>();
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size: (8, 8),
        base: image.pixels.clone(),
        annotations: Some(overlay.clone().into()),
    })
    .unwrap();
    let source = editor.layers().unwrap().clone();
    editor
        .crop_to(Rect {
            x: 4.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let output = tests::decode_png(encode(&image, &editor).unwrap());
    for y in 0..4usize {
        for x in 0..4usize {
            let expected = if x < 2 {
                [0, 0, 200, 255]
            } else {
                [0, 200, 0, 255]
            };
            assert_eq!(
                &output.pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4],
                &expected
            );
        }
    }
    assert!(editor.layers().unwrap().same_storage(&source));
    assert!(editor.reset_crop());
    let reset = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&reset.pixels[..], &overlay[..]);
    assert!(editor.undo());
    let undo = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&undo.pixels[..], &output.pixels[..]);
}

#[test]
fn flipped_crop_export_uses_full_source_mirror_and_preserves_crop_history() {
    let _apartment = Apartment::new().unwrap();
    let (image, _) = document();
    for axis in [
        crate::raster_flip::Axis::Horizontal,
        crate::raster_flip::Axis::Vertical,
    ] {
        let mut editor = Editor::with_layers(Layers {
            annotation_size: None,
            size: (image.width, image.height),
            base: image.pixels.clone(),
            annotations: None,
        })
        .unwrap();
        editor
            .crop_to(Rect {
                x: 3.0,
                y: 5.0,
                width: 11.0,
                height: 7.0,
            })
            .unwrap();
        let before = tests::decode_png(encode(&image, &editor).unwrap());
        let plan = editor
            .prepare_flip(axis, |_| unreachable!())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        // 故意传原始 Image，导出必须读取 Editor 当前栅格，不能用旧画布缓存。
        let output = tests::decode_png(encode(&image, &editor.export_snapshot()).unwrap());
        assert_eq!((output.width, output.height), (11, 7));
        for y in 0..7usize {
            for x in 0..11usize {
                let (sx, sy) = match axis {
                    crate::raster_flip::Axis::Horizontal => (31 - (x + 3), y + 5),
                    crate::raster_flip::Axis::Vertical => (x + 3, 23 - (y + 5)),
                };
                let source = (sy * 32 + sx) * 4;
                let target = (y * 11 + x) * 4;
                assert_eq!(
                    &output.pixels[target..target + 4],
                    &image.pixels[source..source + 4]
                );
            }
        }
        assert!(editor.undo());
        let undone = tests::decode_png(encode(&image, &editor).unwrap());
        assert_eq!(&undone.pixels[..], &before.pixels[..]);
        assert!(editor.redo());
        let redone = tests::decode_png(encode(&image, &editor).unwrap());
        assert_eq!(&redone.pixels[..], &output.pixels[..]);
    }
}

#[test]
fn integer_crop_matches_full_composite_region_and_reset_restores_all_pixels() {
    let _apartment = Apartment::new().unwrap();
    let (image, mut editor) = document();
    let full = tests::decode_png(encode(&image, &editor).unwrap());
    editor
        .crop_to(Rect {
            x: 5.0,
            y: 3.0,
            width: 22.0,
            height: 18.0,
        })
        .unwrap();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 16.0,
            height: 12.0,
        })
        .unwrap();
    let snapshot = editor.export_snapshot();
    let cropped = tests::decode_png(encode(&image, &snapshot).unwrap());
    assert_eq!((cropped.width, cropped.height), (16, 12));
    for y in 0..12usize {
        let start = ((y + 5) * 32 + 7) * 4;
        assert_eq!(
            &cropped.pixels[y * 16 * 4..(y + 1) * 16 * 4],
            &full.pixels[start..start + 16 * 4]
        );
    }
    assert!(editor.reset_crop());
    let restored = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&restored.pixels[..], &full.pixels[..]);
    assert_eq!(restored.width, 32);
    assert!(editor.undo());
    let undone = tests::decode_png(encode(&image, &editor).unwrap());
    assert_eq!(&undone.pixels[..], &cropped.pixels[..]);
}

#[test]
fn crop_keeps_blur_and_mosaic_sampling_in_source_coordinates() {
    let _apartment = Apartment::new().unwrap();
    let (image, mut editor) = document();
    let source = crate::raster_source::compose(editor.layers().unwrap(), || false).unwrap();
    let blurred = crate::blur::region_precise(
        &source,
        (32, 24),
        crate::blur::Region {
            x: 0,
            y: 0,
            width: 32,
            height: 24,
        },
        2.4,
        || false,
    )
    .unwrap();
    let blur = crate::blur_image::BlurImage::with_strength(32, 24, blurred, 2.4).unwrap();
    editor.set_style(editor.style().with_width(8.0).unwrap());
    editor.begin_blur(Point { x: 12.0, y: 12.0 }, blur).unwrap();
    editor.extend(Point { x: 18.0, y: 12.0 }).unwrap();
    editor.commit();
    editor
        .begin(Tool::Mosaic, Point { x: 22.0, y: 16.0 })
        .unwrap();
    editor.commit();
    let full = tests::decode_png(encode(&image, &editor).unwrap());
    editor
        .crop_to(Rect {
            x: 6.0,
            y: 4.0,
            width: 22.0,
            height: 17.0,
        })
        .unwrap();
    let cropped = tests::decode_png(encode(&image, &editor).unwrap());
    for y in 0..17usize {
        let start = ((y + 4) * 32 + 6) * 4;
        assert_eq!(
            &cropped.pixels[y * 22 * 4..(y + 1) * 22 * 4],
            &full.pixels[start..start + 22 * 4]
        );
    }
}

#[test]
fn fractional_crop_rounds_dimensions_and_snapshot_ignores_later_edits() {
    let _apartment = Apartment::new().unwrap();
    let image = Image {
        width: 16,
        height: 16,
        pixels: [20, 40, 60, 255].repeat(256).into(),
    };
    let mut editor = Editor::with_layers(Layers {
        annotation_size: None,
        size: (16, 16),
        base: image.pixels.clone(),
        annotations: None,
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 2.25,
            y: 3.75,
            width: 7.6,
            height: 5.4,
        })
        .unwrap();
    let snapshot = editor.export_snapshot();
    editor.reset_crop();
    editor.begin(Tool::Brush, Point { x: 5.0, y: 5.0 }).unwrap();
    editor.commit();
    let output = tests::decode_png(encode(&image, &snapshot).unwrap());
    assert_eq!((output.width, output.height), (8, 5));
    assert!(output
        .pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [20, 40, 60, 255]));
    assert_eq!(editor.layers().unwrap().base.len(), 16 * 16 * 4);
}
