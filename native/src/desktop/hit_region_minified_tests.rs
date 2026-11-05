//! 用真实 GDI 区域查询确认缩略图片、独立标注和矢量坐标，而不是模拟命中。
use super::*;
use crate::viewport::Point;

#[test]
fn minified_native_regions_follow_each_layer_and_clip_to_client() {
    let mut pixels = vec![0; 32 * 32 * 4];
    for y in 4..8 {
        for x in 4..8 {
            pixels[(y * 32 + x) * 4 + 3] = 255;
        }
    }
    let image = super::super::super::image::Image {
        width: 32,
        height: 32,
        pixels: pixels.into(),
    };
    let mut overlay = vec![0; 32 * 32 * 4];
    for y in 20..24 {
        for x in 20..24 {
            overlay[(y * 32 + x) * 4 + 3] = 255;
        }
    }
    let overlay: crate::pixels::Pixels = overlay.into();
    let cache = HitRegion::prepare_sized(&image, Some(&overlay), (32, 32)).unwrap();
    let views = Views {
        image: View {
            size: Size {
                width: 32.0,
                height: 32.0,
            },
            offset: (2.0, 2.0),
        },
        annotations: View {
            size: Size {
                width: 32.0,
                height: 32.0,
            },
            offset: (12.0, 12.0),
        },
    };
    let geometry = (
        views,
        Size {
            width: 16.0,
            height: 16.0,
        },
    );
    let mut editor = crate::editor::Editor::default();
    for (w, h) in [(16, 16), (24, 24), (32, 32), (24, 32)] {
        unsafe {
            let region = build(&cache, &image, &editor, geometry, (w, h)).unwrap();
            let contains = |x: f64, y: f64| {
                PtInRegion(
                    region.0,
                    (x * w as f64 / 16.0) as i32,
                    (y * h as f64 / 16.0) as i32,
                )
                .as_bool()
            };
            assert!(contains(3.0, 3.0));
            assert!(contains(9.0, 9.0));
            assert!(!contains(14.0, 2.0));
            assert!(!contains(21.0, 21.0));
            assert!(!contains(-1.0, 3.0));
        }
    }
    editor
        .begin(Tool::Brush, Point { x: 26.0, y: 14.0 })
        .unwrap();
    editor.commit();
    unsafe {
        let region = build(&cache, &image, &editor, geometry, (32, 32)).unwrap();
        assert!(PtInRegion(region.0, 28, 4).as_bool());
        assert!(!PtInRegion(region.0, 32, 4).as_bool());
    }
    assert!(cache.source.as_ref().unwrap().0.same_storage(&image.pixels));
    assert!(cache.annotations.as_ref().unwrap().0.same_storage(&overlay));
    let mut scaled = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
        size: (32, 32),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    scaled
        .begin(Tool::Brush, Point { x: 13.0, y: 14.0 })
        .unwrap();
    scaled.commit();
    let wide = Views {
        annotations: View {
            size: Size {
                width: 64.0,
                height: 32.0,
            },
            ..views.annotations
        },
        ..views
    };
    unsafe {
        let region = build(&cache, &image, &scaled, (wide, geometry.1), (16, 16)).unwrap();
        assert!(PtInRegion(region.0, 14, 2).as_bool());
        assert!(!PtInRegion(region.0, 0, 2).as_bool());
    }
}
