//! 独立整数像素oracle；当前legacy区域优先、post-flip一次性和两个裁剪空间。
use super::*;
#[path = "export_projection_legacy_density_tests.rs"]
mod density;

fn region(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}
fn own(image: &Image, pixels: Rect) -> Editor {
    editor(image)
        .with_restored_size(Size {
            width: 8.0,
            height: 8.0,
        })
        .unwrap()
        .with_restored_legacy_crop(
            pixels,
            Size {
                width: 8.0,
                height: 8.0,
            },
        )
        .unwrap()
}
fn assert_pixels(image: &Image, expected: impl Fn(u32, u32) -> [u8; 4]) {
    for y in 0..image.height {
        for x in 0..image.width {
            let at = ((y * image.width + x) * 4) as usize;
            assert_eq!(&image.pixels[at..at + 4], &expected(x, y), "{x},{y}");
        }
    }
}

#[test]
fn projection_explicit_legacy_uses_post_flip_owner_without_reflecting_source_twice() {
    let _com = Apartment::new().unwrap();
    let mut image = patterned();
    // 独立构造已物化水平flip的像素，不调用生产raster_flip作为预期。
    image.pixels = (0..64)
        .flat_map(|i| [((7 - i % 8) * 24) as u8, ((i / 8) * 24) as u8, 32, 255])
        .collect::<Vec<_>>()
        .into();
    let pixels = region(4.0, 0.0, 2.0, 8.0);
    let target = source(
        json!({"flippedX":true,"cropRect":{"x":0,"y":0,"w":1,"h":1}}),
        json!(null),
    );
    let output = render(
        &target,
        &image,
        &own(&image, pixels),
        Input {
            legacy_pixels: Some(pixels),
            content: true,
            legacy_crop: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (8, 8));
    assert_pixels(&output, |x, y| {
        [((3 - x / 4) * 24) as u8, (y * 24) as u8, 32, 255]
    });
}

#[test]
fn projection_explicit_legacy_remains_active_when_old_layout_is_already_materialized() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let pixels = region(2.0, 0.0, 2.0, 8.0);
    let target = source(
        json!({"flippedX":true,"cropRect":{"x":7,"y":0,"w":1,"h":1}}),
        json!({"upstreamSourceFrame":{"w":8,"h":8},"upstreamContentFrame":{"x":4,"y":0,"w":4,"h":8}}),
    );
    let output = render(
        &target,
        &image,
        &own(&image, pixels),
        Input {
            legacy_pixels: Some(pixels),
            content: false,
            legacy_crop: true,
            ..Input::default()
        },
    );
    assert_pixels(&output, |x, y| {
        [((2 + x / 4) * 24) as u8, (y * 24) as u8, 32, 255]
    });
}

#[test]
fn projection_explicit_legacy_keeps_propagated_destination_direction_without_reflecting_pixels() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let pixels = region(2.0, 0.0, 2.0, 8.0);
    let target = source(
        json!({"flippedX":true}),
        json!({"upstreamSourceFrame":{"w":8,"h":8},"upstreamContentFrame":{"x":4,"y":0,"w":4,"h":8}}),
    );
    let output = render(
        &target,
        &image,
        &own(&image, pixels),
        Input {
            legacy_pixels: Some(pixels),
            content: true,
            ..Input::default()
        },
    );
    assert_pixels(&output, |x, y| {
        if x >= 4 {
            [0; 4]
        } else {
            [((2 + x / 2) * 24) as u8, (y * 24) as u8, 32, 255]
        }
    });
}

#[test]
fn projection_own_legacy_nested_crop_is_not_a_modern_contain_or_applied_twice() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let mut editor = own(&image, region(2.0, 0.0, 2.0, 8.0));
    assert!(editor.crop_to(region(2.0, 2.0, 4.0, 4.0)).unwrap());
    let stamp = editor.document_stamp();
    let output = render(
        &source(json!(null), json!(null)),
        &image,
        &editor,
        Input {
            legacy_pixels: editor.crop_view().unwrap().legacy_pixel_region(),
            source_logical: Some(Size {
                width: 16.0,
                height: 4.0,
            }),
            content: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (4, 4));
    assert_pixels(&output, |x, y| {
        [((2 + x / 2) * 24) as u8, ((2 + y) * 24) as u8, 32, 255]
    });
    assert!(stamp.matches(&editor));
    assert!(editor.undo());
    assert_eq!(
        editor.crop_view().unwrap().legacy_pixel_region(),
        Some(region(2.0, 0.0, 2.0, 8.0))
    );
}

#[test]
fn projection_upstream_legacy_and_target_modern_crop_keep_separate_spaces() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let mut editor = editor(&image);
    editor.crop_to(region(2.0, 2.0, 4.0, 4.0)).unwrap();
    assert!(editor.crop_view().unwrap().legacy_pixel_region().is_none());
    let output = render(
        &source(json!(null), json!(null)),
        &image,
        &editor,
        Input {
            legacy_pixels: Some(region(2.0, 0.0, 2.0, 8.0)),
            source_logical: Some(Size {
                width: 8.0,
                height: 8.0,
            }),
            content: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (4, 4));
    assert_pixels(&output, |x, y| {
        [((2 + x / 2) * 24) as u8, ((2 + y) * 24) as u8, 32, 255]
    });
}

#[test]
fn projection_explicit_legacy_preserves_partial_and_empty_source_as_transparency() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    for pixels in [region(-2.0, 0.0, 4.0, 8.0), region(20.0, 0.0, 4.0, 8.0)] {
        let output = render(
            &source(json!(null), json!(null)),
            &image,
            &own(&image, pixels),
            Input {
                legacy_pixels: Some(pixels),
                ..Input::default()
            },
        );
        assert_pixels(&output, |x, y| {
            if pixels.x > 8.0 || x < 4 {
                [0; 4]
            } else {
                [((x - 4) / 2 * 24) as u8, (y * 24) as u8, 32, 255]
            }
        });
    }
}

#[test]
fn projection_explicit_legacy_rejects_invalid_geometry_without_mutating_owner() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let editor = editor(&image);
    let before = editor.document_stamp();
    let target = source(json!(null), json!(null));
    for pixels in [
        region(f64::NAN, 0.0, 1.0, 1.0),
        region(0.0, 0.0, 0.0, 1.0),
        region(1_000_001.0, 0.0, 1.0, 1.0),
    ] {
        let error = target_frame_with_input(
            &target,
            "target",
            &image,
            &editor,
            Presentation {
                opacity: 1.0,
                kind: crate::projection_delivery::sender::Kind::Sticker,
                geometry: None,
            },
            Input {
                legacy_pixels: Some(pixels),
                ..Input::default()
            },
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), E_INVALIDARG);
    }
    assert!(before.matches(&editor));
    assert!(editor.layers().unwrap().base.same_storage(&image.pixels));
}
