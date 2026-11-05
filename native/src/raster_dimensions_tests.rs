//! 独立标注纹理尺寸的底层合同；宿主接入前不允许静默按底图尺寸解释该层。
use super::*;
fn layers() -> Layers {
    Layers {
        annotation_size: Some((2, 1)),
        size: (8, 4),
        base: vec![0; 128].into(),
        annotations: Some(vec![0, 0, 255, 255, 0, 255, 0, 255].into()),
    }
}
#[test]
fn dimension_identity_and_validation_include_shape_not_only_byte_length() {
    let source = layers();
    assert!(source.valid());
    let mut other = source.clone();
    other.annotation_size = Some((1, 2));
    assert!(other.valid());
    assert!(!other.same_storage(&source));
    other.annotation_size = Some((3, 1));
    assert!(!other.valid());
    other.annotation_size = Some((0, 1));
    assert!(!other.valid());
    other.annotations = None;
    assert!(!other.valid());
    other.annotation_size = None;
    assert!(other.valid());
}
#[test]
fn independent_overlay_composition_and_mirror_preserve_base_size() {
    let source = layers();
    let full = crate::raster_source::compose(&source, || false).unwrap();
    assert_eq!(full.len(), 128);
    assert_eq!(&full[0..4], &[0, 0, 255, 255]);
    assert_eq!(&full[28..32], &[0, 255, 0, 255]);
    let flipped =
        crate::raster_flip::layers(&source, crate::raster_flip::Axis::Horizontal, || false)
            .unwrap();
    assert_eq!(flipped.size, (8, 4));
    assert_eq!(flipped.annotation_dimensions(), (2, 1));
    assert_eq!(
        &flipped.annotations.as_ref().unwrap()[..],
        &[0, 255, 0, 255, 0, 0, 255, 255]
    );
    let back = crate::raster_flip::layers(&flipped, crate::raster_flip::Axis::Horizontal, || false)
        .unwrap();
    assert_eq!(back.annotations, source.annotations);
}
#[test]
fn independent_overlay_erasure_maps_logical_frame_and_retains_dimensions() {
    let mut source = layers();
    source.annotation_size = Some((32, 16));
    source.annotations = Some(vec![255; 32 * 16 * 4].into());
    let erased = source
        .erase_in_frame(
            Mode::RasterAnnotations,
            &[Point { x: 6.0, y: 2.0 }],
            4.0,
            Some(crate::crop_geometry::Rect {
                x: 4.0,
                y: 0.0,
                width: 4.0,
                height: 4.0,
            }),
            || false,
        )
        .unwrap();
    assert_eq!(erased.annotation_dimensions(), (32, 16));
    assert!(erased.base.same_storage(&source.base));
    assert_eq!(
        erased.annotations.as_ref().unwrap()[(8 * 32 + 16) * 4 + 3],
        0
    );
    assert_eq!(
        source.annotations.as_ref().unwrap()[(8 * 32 + 16) * 4 + 3],
        255
    );
}
