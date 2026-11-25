//! 独立坐标/像素oracle覆盖完整底图映射、擦除与效果事务，不以投送实现生成预期。
use super::*;

fn editor() -> Editor {
    let layers = crate::erase_layers::Layers {
        size: (8, 8),
        base: (0..64)
            .flat_map(|i| {
                let value = if (1..=4).contains(&(i % 8)) { 64 } else { 192 };
                [value, value, value, 255]
            })
            .collect::<Vec<_>>()
            .into(),
        annotations: None,
        annotation_size: None,
    };
    Editor::with_layers(layers)
        .unwrap()
        .with_restored_size(Size {
            width: 8.0,
            height: 8.0,
        })
        .unwrap()
        .with_restored_legacy_crop(
            Rect {
                x: 2.0,
                y: 0.0,
                width: 2.0,
                height: 8.0,
            },
            Size {
                width: 8.0,
                height: 8.0,
            },
        )
        .unwrap()
}

#[test]
fn legacy_mapping_keeps_full_texture_and_nested_crop_only_changes_clip() {
    let mut editor = editor();
    let frame = crate::image_projection::Projection {
        source: Rect {
            x: 2.0,
            y: 0.0,
            width: 2.0,
            height: 8.0,
        },
        target: Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
        },
    };
    assert_eq!(editor.raster_base_projection(), Some(frame));
    let source = editor.layers().unwrap().base.clone();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    assert_eq!(editor.raster_base_projection(), Some(frame));
    assert!(editor.layers().unwrap().base.same_storage(&source));
    assert!(editor.reset_crop());
    assert_eq!(editor.raster_base_projection(), None);
    assert!(editor.undo());
    assert_eq!(editor.raster_base_projection(), Some(frame));
}

#[test]
fn natural_non_square_source_accounts_for_original_contain_padding() {
    let mut view = editor().crop_view().unwrap();
    view.legacy_natural_size = Some(Size {
        width: 16.0,
        height: 8.0,
    });
    view.legacy_pixels = Some(Rect {
        x: 4.0,
        y: 0.0,
        width: 4.0,
        height: 8.0,
    });
    // 16x8原图位于8x8底图的y=2..6；选区位于x=2..4，分别放大4与2倍。
    assert_eq!(
        view.base_projection((8, 8)),
        Some(crate::image_projection::Projection {
            source: Rect {
                x: 2.0,
                y: 2.0,
                width: 2.0,
                height: 4.0
            },
            target: Rect {
                x: 0.0,
                y: 0.0,
                width: 8.0,
                height: 8.0
            },
        })
    );
}

#[test]
fn edited_backing_texture_save_uses_its_own_pixels_not_retired_natural_coordinates() {
    let mut editor = editor();
    let view = editor.crop.as_mut().unwrap();
    view.legacy_natural_size = Some(Size {
        width: 16.0,
        height: 8.0,
    });
    view.legacy_pixels = Some(Rect {
        x: 4.0,
        y: 0.0,
        width: 4.0,
        height: 8.0,
    });
    let saved = editor.raster_crop_view().unwrap();
    assert_eq!(
        saved.legacy_pixel_region(),
        Some(Rect {
            x: 2.0,
            y: 2.0,
            width: 2.0,
            height: 4.0
        })
    );
    assert_eq!(
        saved.base_projection((8, 8)),
        editor.raster_base_projection()
    );
    assert_eq!(
        editor.crop_view().unwrap().legacy_pixel_region().unwrap().x,
        4.0
    );
}

#[test]
fn legacy_effect_source_samples_region_without_changing_original_or_ignoring_cancel() {
    let editor = editor();
    let layers = editor.layers().unwrap();
    let output = crate::raster_source::compose_in_frames(
        layers,
        editor.raster_base_projection(),
        editor.annotation_frame(),
        || false,
    )
    .unwrap();
    assert_eq!(&output[..], &[64, 64, 64, 255].repeat(64));
    assert_eq!(&layers.base[..4], &[192, 192, 192, 255]);
    assert!(matches!(
        crate::raster_source::compose_in_frames(
            layers,
            editor.raster_base_projection(),
            editor.annotation_frame(),
            || true,
        ),
        Err(crate::blur::BlurError::Cancelled)
    ));
}

#[test]
fn legacy_erase_uses_document_brush_width_and_inverse_texture_mapping() {
    let editor = editor();
    let layers = editor.layers().unwrap();
    let erased = layers
        .erase_in_frames(
            crate::erase_layers::Mode::Content,
            &[Point { x: 4.0, y: 4.0 }],
            8.0,
            editor.raster_base_projection(),
            editor.annotation_frame(),
            || false,
        )
        .unwrap();
    assert_eq!(erased.base[(4 * 8 + 2) * 4 + 3], 0);
    assert_eq!(erased.base[(4 * 8) * 4 + 3], 255);
    assert_eq!(layers.base[(4 * 8 + 2) * 4 + 3], 255);
}

#[test]
fn legacy_reset_recomputes_blur_without_annotations_and_undo_restores_it_atomically() {
    let mut editor = editor();
    let image =
        crate::blur_image::BlurImage::with_strength(8, 8, [64, 64, 64, 255].repeat(64).into(), 2.0)
            .unwrap();
    editor.begin_blur(Point { x: 4.0, y: 4.0 }, image).unwrap();
    editor.commit();
    let before = editor.elements()[0].blur.clone().unwrap();
    let plan = editor.prepare_crop_refresh(None).unwrap().unwrap();
    assert!(plan.requires_effect_compute());
    let plan = plan.compute(|| false).unwrap();
    assert!(editor.commit_raster_refresh(plan).unwrap());
    assert!(editor.crop_view().is_none());
    assert!(editor.undo());
    assert!(editor.raster_base_projection().is_some());
    assert_eq!(
        editor.elements()[0].blur.as_ref().unwrap().pixels(),
        before.pixels()
    );
}
