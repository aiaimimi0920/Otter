use super::*;
#[test]
fn worker_mask_retains_independent_layer_identity_without_gdi_handles() {
    let image = super::super::image::Image {
        width: 4,
        height: 2,
        pixels: vec![255; 32].into(),
    };
    let annotations: crate::pixels::Pixels = vec![255; 16].into();
    let region =
        HitRegion::prepare_cancellable(&image, Some(&annotations), (2, 2), || false).unwrap();
    assert!(region
        .source
        .as_ref()
        .unwrap()
        .0
        .same_storage(&image.pixels));
    assert!(region
        .annotations
        .as_ref()
        .unwrap()
        .0
        .same_storage(&annotations));
    assert_eq!(region.annotation_size, (2, 2));
    assert!(region.key.is_none());
    assert!(HitRegion::prepare_cancellable(&image, Some(&annotations), (2, 2), || true).is_err());
}
#[test]
fn region_key_invalidates_for_view_changes_without_document_or_window_changes() {
    let original = Key {
        source: 1,
        revision: 2,
        pan: true,
        client: (640, 480),
        dpi: 144,
        viewport: crate::viewport::Viewport {
            origin: crate::viewport::Point { x: -30.0, y: 20.0 },
            scale: 1.5,
        },
        minified: None,
        base_projection: None,
        style: Default::default(),
        overlay: 0,
    };
    let mut moved = original;
    moved.viewport.origin.x += 1.0;
    assert_ne!(original, moved);
    let mut zoomed = original;
    zoomed.viewport.scale = 2.0;
    assert_ne!(original, zoomed);
    let mut vertical = original;
    vertical.viewport.origin.y -= 1.0;
    assert_ne!(original, vertical);
    assert_eq!(original, Key { ..original });
}

#[test]
fn independent_annotation_dimensions_map_native_mask_and_reject_wrong_stride() {
    let image = super::super::image::Image {
        width: 32,
        height: 32,
        pixels: vec![0; 4096].into(),
    };
    let mut pixels = vec![0; 8 * 4 * 4];
    pixels[(8 + 1) * 4 + 3] = 255;
    let pixels: crate::pixels::Pixels = pixels.into();
    let horizontal = HitRegion::prepare_sized(&image, Some(&pixels), (8, 4)).unwrap();
    let vertical = HitRegion::prepare_sized(&image, Some(&pixels), (4, 8)).unwrap();
    let crop = Some(crate::crop_geometry::Rect {
        x: 16.0,
        y: 8.0,
        width: 16.0,
        height: 16.0,
    });
    unsafe {
        let h = raster_region(&horizontal, 32, 32, 1.0, crop, None).unwrap();
        assert!(PtInRegion(h.0, 19, 14).as_bool());
        assert!(!PtInRegion(h.0, 30, 14).as_bool());
        let v = raster_region(&vertical, 32, 32, 1.0, crop, None).unwrap();
        assert!(PtInRegion(v.0, 22, 13).as_bool());
        assert!(!PtInRegion(v.0, 22, 20).as_bool());
    }
    assert!(HitRegion::prepare_sized(&image, Some(&pixels), (8, 8)).is_err());
    assert!(HitRegion::prepare_sized(&image, Some(&pixels), (0, 4)).is_err());
}

#[test]
fn raster_annotation_has_native_hit_region_when_base_is_erased() {
    let image = super::super::image::Image {
        width: 16,
        height: 16,
        pixels: vec![0; 16 * 16 * 4].into(),
    };
    let mut pixels = vec![0; 16 * 16 * 4];
    pixels[(8 * 16 + 8) * 4 + 3] = 1;
    let annotations: crate::pixels::Pixels = pixels.into();
    let cache = HitRegion::prepare_layers(&image, Some(&annotations)).unwrap();
    unsafe {
        let region = raster_region(&cache, 16, 16, 1.0, None, None).unwrap();
        assert!(PtInRegion(region.0, 8, 8).as_bool());
        assert!(!PtInRegion(region.0, 2, 2).as_bool());
    }
    assert!(HitRegion::prepare(&image)
        .unwrap()
        .source
        .unwrap()
        .1
        .rectangles
        .is_empty());
}
#[test]
fn cropped_overlay_hit_region_moves_independently_from_base_at_multiple_scales() {
    let mut base = vec![0; 32 * 32 * 4];
    for y in 24..28 {
        for x in 24..28 {
            base[(y * 32 + x) * 4 + 3] = 255;
        }
    }
    let image = super::super::image::Image {
        width: 32,
        height: 32,
        pixels: base.into(),
    };
    let mut overlay = vec![0; 32 * 32 * 4];
    for y in 4..12 {
        for x in 4..12 {
            overlay[(y * 32 + x) * 4 + 3] = 255;
        }
    }
    let pixels: crate::pixels::Pixels = overlay.into();
    let cache = HitRegion::prepare_layers(&image, Some(&pixels)).unwrap();
    let frame = crate::crop_geometry::Rect {
        x: 16.0,
        y: 4.0,
        width: 16.0,
        height: 8.0,
    };
    for scale in [1.0, 1.5, 2.0] {
        unsafe {
            let region = raster_region(&cache, 32, 32, scale, Some(frame), None).unwrap();
            assert!(PtInRegion(region.0, (20.0 * scale) as i32, (6.0 * scale) as i32).as_bool());
            assert!(!PtInRegion(region.0, (8.0 * scale) as i32, (8.0 * scale) as i32).as_bool());
            // The base image remains at its original position; update's shared clip handles final visible cropping.
            assert!(PtInRegion(region.0, (26.0 * scale) as i32, (26.0 * scale) as i32).as_bool());
            let reset = raster_region(&cache, 32, 32, scale, None, None).unwrap();
            assert!(PtInRegion(reset.0, (8.0 * scale) as i32, (8.0 * scale) as i32).as_bool());
            assert!(!PtInRegion(reset.0, (20.0 * scale) as i32, (6.0 * scale) as i32).as_bool());
        }
    }
    assert!(cache.annotations.as_ref().unwrap().0.same_storage(&pixels));
}

#[test]
fn native_region_preserves_holes_and_scaled_sampling_border() {
    let mut pixels = vec![0; 12 * 12 * 4];
    pixels[(6 * 12 + 6) * 4 + 3] = 255;
    let mask = alpha_region::build(12, 12, &pixels).unwrap();
    unsafe {
        let region = source_region(&mask, 12, 12, 2.0).unwrap();
        assert!(!PtInRegion(region.0, 1, 1).as_bool());
        assert!(PtInRegion(region.0, 12, 12).as_bool());
        assert!(!PtInRegion(region.0, 20, 20).as_bool());
    }
}

#[test]
fn projected_hit_region_matches_selected_pixels_not_unmapped_alpha() {
    use crate::crop_geometry::Rect;
    let mut pixels = vec![0; 64 * 8 * 4];
    pixels[(3 * 64 + 2) * 4 + 3] = 255;
    pixels[(3 * 64 + 42) * 4 + 3] = 255;
    let image = super::super::image::Image {
        width: 64,
        height: 8,
        pixels: pixels.into(),
    };
    let cache = HitRegion::prepare(&image).unwrap();
    let source = Rect {
        x: 40.0,
        y: 2.0,
        width: 8.0,
        height: 4.0,
    };
    let target = Rect {
        x: 16.0,
        y: 4.0,
        width: 64.0,
        height: 16.0,
    };
    for scale in [1.0, 1.5, 2.0] {
        unsafe {
            let mapping = Projection { source, target };
            let region = raster_region(&cache, 64, 8, scale, None, Some(mapping)).unwrap();
            let hit = |x: f32, y: f32| {
                PtInRegion(region.0, (x * scale) as i32, (y * scale) as i32).as_bool()
            };
            assert!(hit(36.0, 10.0));
            assert!(!hit(76.0, 10.0));
            assert!(!hit(42.0, 3.0));
            assert!(!hit(2.0, 3.0));

            let outside = Projection {
                source: Rect { x: 80.0, ..source },
                target,
            };
            let empty = raster_region(&cache, 64, 8, scale, None, Some(outside)).unwrap();
            assert!(!PtInRegion(empty.0, (36.0 * scale) as i32, (10.0 * scale) as i32).as_bool());
        }
    }
}

#[test]
fn annotation_bounds_remain_interactive_on_transparent_source() {
    let image = super::super::image::Image {
        width: 32,
        height: 32,
        pixels: vec![0; 32 * 32 * 4].into(),
    };
    let mut canvas = super::super::canvas::Canvas::new(Some(image)).unwrap();
    canvas.viewport.origin = crate::viewport::Point::default();
    canvas
        .editor
        .begin(Tool::Brush, crate::viewport::Point { x: 16.0, y: 16.0 })
        .unwrap();
    canvas.editor.commit();
    unsafe {
        let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
        add_annotations(&region, &canvas.editor, canvas.viewport.scale, 96).unwrap();
        assert!(PtInRegion(region.0, 16, 16).as_bool());
        assert!(!PtInRegion(region.0, 2, 2).as_bool());
    }
}
