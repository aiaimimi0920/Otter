//! 显式历史空间的独立椭圆覆盖oracle、Empty和失败交换原子性。
use super::*;
use serde_json::json;

fn image() -> Image {
    Image {
        width: 8,
        height: 8,
        pixels: [20, 40, 60, 255].repeat(64).into(),
    }
}
fn geometry() -> Frame {
    let mut geometry: Frame = Size {
        width: 8.0,
        height: 8.0,
    }
    .into();
    geometry.sampling = Projection::clipped(
        Rect {
            x: 2.0,
            y: 0.0,
            width: 2.0,
            height: 8.0,
        },
        Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 8.0,
        },
        (8, 8),
    )
    .unwrap();
    geometry
}
fn state(geometry: &Frame) -> String {
    format!(
        "{:?}/{:?}/{:?}/{:?}",
        geometry.output, geometry.base, geometry.base_clip, geometry.sampling
    )
}
fn map_state(mapping: &Sampling) -> String {
    format!("{mapping:?}")
}

#[test]
fn mapped_erase_preserves_nonuniform_history_brush_with_independent_subpixel_coverage() {
    let image = image();
    let mut geometry: Frame = Size {
        width: 8.0,
        height: 4.0,
    }
    .into();
    let mut mapping = Sampling::Region(Projection {
        source: bitmap_rect(&image),
        target: Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 16.0,
        },
    });
    let result = prepare_mapped(
        &image,
        &mut geometry,
        None,
        &json!({"contentEraseStrokes":[{"width":4,"points":[{"x":2,"y":8}]}]}),
        &mut mapping,
        &AtomicBool::new(false),
    )
    .unwrap();
    for y in 0..8usize {
        for x in 0..8usize {
            let mut removed = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let hx = (x as f64 + (sx as f64 + 0.5) / 4.0) * 0.5;
                    let hy = (y as f64 + (sy as f64 + 0.5) / 4.0) * 2.0;
                    if (hx - 2.0).powi(2) + (hy - 8.0).powi(2) <= 4.0 {
                        removed += 1;
                    }
                }
            }
            for (actual, base) in result.pixels[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4]
                .iter()
                .zip([20u32, 40, 60, 255])
            {
                assert_eq!(*actual, ((base * (16 - removed) + 8) / 16) as u8, "{x},{y}");
            }
        }
    }
    assert!(image.pixels.chunks_exact(4).all(|p| p == [20, 40, 60, 255]));
}

#[test]
fn mapped_erase_empty_mapping_checks_history_without_materializing_or_changing_geometry() {
    let image = image();
    let mut geometry = geometry();
    let before = state(&geometry);
    let mut mapping = Sampling::Empty;
    let output = prepare_mapped(
        &image,
        &mut geometry,
        None,
        &json!({"contentEraseStrokes":[{"width":2,"points":[{"x":2,"y":2}]}]}),
        &mut mapping,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(output.pixels.same_storage(&image.pixels));
    assert_eq!(state(&geometry), before);
    assert!(matches!(mapping, Sampling::Empty));
    assert!(prepare_mapped(
        &image,
        &mut geometry,
        None,
        &json!({"contentEraseStrokes":[{"width":0,"points":[{"x":2,"y":2}]}]}),
        &mut mapping,
        &AtomicBool::new(false)
    )
    .is_err());
    assert_eq!(state(&geometry), before);
}

#[test]
fn mapped_erase_failure_or_cancel_never_exchanges_partially_sampled_geometry_or_mapping() {
    let image = image();
    let mut geometry = geometry();
    let before = state(&geometry);
    let mut mapping = Sampling::Full;
    let original = map_state(&mapping);
    let edit = json!({"contentEraseStrokes":[{"width":2,"points":[{"x":2,"y":2}]},
        {"width":0,"points":[{"x":6,"y":6}]}]});
    for cancel in [false, true] {
        assert!(prepare_mapped(
            &image,
            &mut geometry,
            None,
            &edit,
            &mut mapping,
            &AtomicBool::new(cancel)
        )
        .is_err());
        assert_eq!(state(&geometry), before);
        assert_eq!(map_state(&mapping), original);
        assert!(image.pixels.chunks_exact(4).all(|p| p == [20, 40, 60, 255]));
    }
    let mut leases = Vec::new();
    let mut bytes = crate::pixels::DECODED_PIXEL_LIMIT;
    while bytes > 0 {
        while let Some(lease) = crate::pixels::reserve(bytes) {
            leases.push(lease);
        }
        bytes /= 2;
    }
    let error = prepare_mapped(
        &image,
        &mut geometry,
        None,
        &json!({"contentEraseStrokes":[{"width":2,"points":[{"x":2,"y":2}]}]}),
        &mut mapping,
        &AtomicBool::new(false),
    )
    .err()
    .unwrap();
    assert_eq!(error.code(), windows::Win32::Foundation::E_OUTOFMEMORY);
    assert_eq!(state(&geometry), before);
    assert_eq!(map_state(&mapping), original);
    drop(leases);
}
