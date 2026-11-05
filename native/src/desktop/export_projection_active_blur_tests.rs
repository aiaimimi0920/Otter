//! 活动blur以独立高斯公式和像素锚点验证源密度、裁剪邻域与失败原子性。
use super::*;
use crate::{
    blur_image::BlurImage,
    crop_geometry::{Rect, Size},
};
#[path = "export_projection_active_blur_layout_tests.rs"]
mod layout;
#[path = "export_projection_active_blur_sampling_tests.rs"]
mod sampling;

fn blur_editor() -> Editor {
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (31, 31),
        base: vec![0; 31 * 31 * 4].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor
        .begin_blur(
            Point { x: 15.0, y: 15.0 },
            BlurImage::new(31, 31, vec![0; 31 * 31 * 4].into(), 1).unwrap(),
        )
        .unwrap();
    assert!(editor.commit());
    editor
}
fn impulse() -> Image {
    let mut pixels = vec![0; 62 * 62 * 4];
    pixels[(31 * 62 + 31) * 4..(31 * 62 + 31) * 4 + 4].fill(255);
    Image {
        width: 62,
        height: 62,
        pixels: pixels.into(),
    }
}
fn weight(offset: i32, sigma: f64) -> f64 {
    let radius = (sigma * 3.0).ceil() as i32;
    if offset.abs() > radius {
        return 0.0;
    }
    let f = |x: i32| (-(x as f64).powi(2) / (2.0 * sigma * sigma)).exp();
    f(offset) / (-radius..=radius).map(f).sum::<f64>()
}

#[test]
fn projection_active_blur_recomputes_natural_density_with_crop_neighborhood_and_exact_strength() {
    let image = impulse();
    let mut editor = blur_editor();
    editor
        .crop_to(Rect {
            x: 10.0,
            y: 10.0,
            width: 10.0,
            height: 10.0,
        })
        .unwrap();
    let before = editor.document_stamp();
    let original = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
    let geometry = source_resolution::Frame::from(Size {
        width: 31.0,
        height: 31.0,
    });
    let prepared = source_resolution::source_blur::prepare_active(
        &image,
        &editor,
        &geometry,
        &AtomicBool::new(false),
    )
    .unwrap();
    let blur = prepared.elements()[0].blur.as_ref().unwrap();
    assert_eq!((blur.width(), blur.height()), (62, 62));
    assert_eq!(
        blur.frame(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 31.0,
            height: 31.0
        }
    );
    for y in 0..62 {
        for x in 0..62 {
            let expected =
                (255.0 * weight(x as i32 - 31, 2.0) * weight(y as i32 - 31, 2.0)).round() as u8;
            assert!(
                blur.pixels()[(y * 62 + x) * 4].abs_diff(expected) <= 1,
                "{x},{y}"
            );
        }
    }
    assert!(before.matches(&editor));
    assert!(editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(&original));
}

#[test]
fn projection_active_blur_no_effects_are_lazy_and_cancellation_keeps_resources_unchanged() {
    let image = impulse();
    let geometry = source_resolution::Frame::from(Size {
        width: 31.0,
        height: 31.0,
    });
    let plain = Editor::default();
    assert!(source_resolution::source_blur::prepare_active(
        &image,
        &plain,
        &geometry,
        &AtomicBool::new(false)
    )
    .unwrap()
    .elements()
    .is_empty());
    let editor = blur_editor();
    let original = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
    assert!(source_resolution::source_blur::prepare_active(
        &image,
        &editor,
        &geometry,
        &AtomicBool::new(true)
    )
    .is_err());
    assert!(editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(&original));
}

#[test]
fn projection_active_blur_rejects_excessive_texture_before_mutating_active_snapshot() {
    let image = impulse();
    let mut editor = Editor::default();
    editor
        .begin_blur(
            Point::default(),
            BlurImage::new(1, 1, vec![255; 4].into(), 1)
                .unwrap()
                .with_frame(Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 1_000_000.0,
                    height: 1_000_000.0,
                })
                .unwrap(),
        )
        .unwrap();
    editor.commit();
    let stamp = editor.document_stamp();
    assert!(source_resolution::source_blur::prepare_active(
        &image,
        &editor,
        &source_resolution::Frame::from(Size {
            width: 31.0,
            height: 31.0
        }),
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(stamp.matches(&editor));
}

#[test]
fn projection_active_blur_budget_failure_releases_partial_work_and_keeps_active_pixels() {
    let image = impulse();
    let editor = blur_editor();
    let stamp = editor.document_stamp();
    let original = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
    // 仅预占现有全局预算，不分配大像素或放宽上限；串行运行后全部租约释放。
    let mut leases = Vec::new();
    let mut bytes = crate::pixels::DECODED_PIXEL_LIMIT;
    while bytes > 0 {
        while let Some(lease) = crate::pixels::reserve(bytes) {
            leases.push(lease);
        }
        bytes /= 2;
    }
    let error = source_resolution::source_blur::prepare_active(
        &image,
        &editor,
        &source_resolution::Frame::from(Size {
            width: 31.0,
            height: 31.0,
        }),
        &AtomicBool::new(false),
    )
    .err()
    .unwrap();
    assert_eq!(error.code(), windows::Win32::Foundation::E_OUTOFMEMORY);
    assert!(stamp.matches(&editor));
    assert!(editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(&original));
    drop(leases);
    assert!(source_resolution::source_blur::prepare_active(
        &image,
        &editor,
        &source_resolution::Frame::from(Size {
            width: 31.0,
            height: 31.0
        }),
        &AtomicBool::new(false),
    )
    .is_ok());
}
