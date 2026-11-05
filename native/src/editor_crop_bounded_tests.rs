//! 极窄legacy区域只保留有界source/target；自然源/工作纹理换基准及取消不丢完整底图。
use super::*;
fn editor(vertical: bool) -> Editor {
    let size = if vertical { (1, 8192) } else { (8192, 1) };
    let logical = Size {
        width: size.0 as f64,
        height: size.1 as f64,
    };
    Editor::with_layers(crate::erase_layers::Layers {
        size,
        base: [20, 40, 60, 255].repeat(8192).into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_size(logical)
    .unwrap()
    .with_restored_legacy_crop(
        Rect {
            x: if vertical { 0.0 } else { 1234.0 },
            y: if vertical { 1234.0 } else { 0.0 },
            width: 1.0,
            height: 1.0,
        },
        logical,
    )
    .unwrap()
}
#[test]
fn narrow_region_install_compose_save_reset_and_undo_never_expand_full_texture_placement() {
    for vertical in [false, true] {
        let mut editor = editor(vertical);
        let source = editor.layers().unwrap().clone();
        let mapping = editor.raster_base_projection().unwrap();
        assert_eq!(mapping.source.width, 1.0);
        assert_eq!(mapping.source.height, 1.0);
        assert_eq!(
            (mapping.target.width, mapping.target.height),
            (source.size.0 as f64, source.size.1 as f64)
        );
        assert_eq!(
            editor.raster_crop_view().unwrap().legacy_pixel_region(),
            Some(mapping.source)
        );
        let output = crate::raster_source::compose_in_frames(
            &source,
            Some(mapping),
            editor.annotation_frame(),
            || false,
        )
        .unwrap();
        assert_eq!(&output[..], &[20, 40, 60, 255].repeat(8192));
        assert!(matches!(
            crate::raster_source::compose_in_frames(
                &source,
                Some(mapping),
                editor.annotation_frame(),
                || true
            ),
            Err(crate::blur::BlurError::Cancelled)
        ));
        assert!(editor.reset_crop());
        assert!(editor.raster_base_projection().is_none());
        assert!(editor.undo());
        assert_eq!(editor.raster_base_projection(), Some(mapping));
        assert!(editor.layers().unwrap().same_storage(&source));
    }
}
#[test]
fn entirely_outside_legacy_region_stays_empty_not_full_and_keeps_source() {
    let mut editor = editor(false);
    editor
        .crop
        .as_mut()
        .unwrap()
        .legacy_pixels
        .as_mut()
        .unwrap()
        .x = 9000.0;
    let layers = editor.layers().unwrap();
    let output = crate::raster_source::compose_in_frames(
        layers,
        editor.raster_base_projection(),
        editor.annotation_frame(),
        || false,
    )
    .unwrap();
    assert!(output.iter().all(|v| *v == 0));
    assert_eq!(&layers.base[..4], &[20, 40, 60, 255]);
    let polls = std::cell::Cell::new(0);
    assert!(matches!(
        crate::raster_source::compose_in_frames(
            layers,
            editor.raster_base_projection(),
            editor.annotation_frame(),
            || {
                polls.set(polls.get() + 1);
                polls.get() > 1
            }
        ),
        Err(crate::blur::BlurError::Cancelled)
    ));
}
