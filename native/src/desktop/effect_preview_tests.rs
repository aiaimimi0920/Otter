//! 真实 D2D 像素与宿主手势回归：效果预览不能移动源图采样坐标。
use super::super::{canvas::Canvas, effect_cache::Cache};
use super::*;
use crate::{blur_image::BlurImage, editor::Tool};

fn canvas(tool: Tool) -> Canvas {
    let pixels = (0..64 * 64)
        .flat_map(|i| [(i % 64 * 4) as u8, (i / 64 * 4) as u8, 0, 255])
        .collect::<Vec<_>>();
    let mut canvas = Canvas::new(Some(Image {
        width: 64,
        height: 64,
        pixels: pixels.into(),
    }))
    .unwrap();
    canvas.viewport = Viewport {
        origin: Point::default(),
        scale: 1.0,
    };
    canvas
        .editor
        .set_style(canvas.editor.style().with_width(12.0).unwrap());
    let start = Point { x: 16.0, y: 24.0 };
    if tool == Tool::Blur {
        let image = canvas.image.as_ref().unwrap();
        let blur = BlurImage::new(64, 64, image.pixels.clone(), 8).unwrap();
        canvas.editor.begin_blur(start, blur).unwrap();
    } else {
        canvas.editor.begin(tool, start).unwrap();
    }
    canvas.editor.extend(Point { x: 36.0, y: 24.0 }).unwrap();
    assert!(canvas.editor.commit());
    canvas.select_tool(Tool::Select);
    canvas
}

fn render(canvas: &Canvas, preview: bool) -> Vec<u8> {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(64, 64, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            dpiX: 96.0,
            dpiY: 96.0,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            ..Default::default()
        };
        let target = canvas
            .factory
            .CreateWicBitmapRenderTarget(&output, &properties)
            .unwrap();
        let brush = target
            .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
            .unwrap();
        let mut cache = Cache::default();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        if preview && !canvas.group_preview.is_empty() {
            annotations::draw_group_preview(
                &target,
                &brush,
                &canvas.editor,
                canvas.viewport,
                &canvas.group_preview,
                &mut cache,
            )
            .unwrap();
        } else if preview {
            annotations::draw_preview(
                &target,
                &brush,
                &canvas.editor,
                canvas.viewport,
                None,
                Some((0, canvas.selection_effect_preview.as_ref().unwrap())),
                &mut cache,
            )
            .unwrap();
        } else {
            annotations::draw_committed(
                &target,
                &brush,
                &canvas.editor,
                canvas.viewport,
                &mut cache,
            )
            .unwrap();
        }
        target.EndDraw(None, None).unwrap();
        let mut bytes = vec![0; 64 * 64 * 4];
        output
            .CopyPixels(std::ptr::null(), 64 * 4, &mut bytes)
            .unwrap();
        bytes
    }
}

#[test]
fn group_drag_preserves_effect_sampling_layer_order_and_single_history() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Blur, Tool::Mosaic] {
        let mut canvas = canvas(tool);
        canvas
            .editor
            .begin(Tool::Highlighter, Point { x: 24.0, y: 42.0 })
            .unwrap();
        canvas.editor.extend(Point { x: 44.0, y: 42.0 }).unwrap();
        assert!(canvas.editor.commit());
        // 反向选择顺序不改变合成顺序，荧光笔仍处于效果下方。
        assert!(canvas.toggle_selection_at(Point { x: 30.0, y: 42.0 }));
        assert!(canvas.toggle_selection_at(Point { x: 24.0, y: 24.0 }));
        assert_eq!(canvas.selected_indices(), vec![0, 1]);
        let committed_before = render(&canvas, false);
        let original: Vec<_> = canvas
            .editor
            .elements()
            .iter()
            .map(|s| s.points.clone())
            .collect();
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(Point { x: 24.0, y: 24.0 }));
        let pointers: Vec<_> = canvas
            .group_preview
            .iter()
            .map(|(_, s)| s.points.as_ptr())
            .collect();
        assert_eq!(pointers.len(), 2);
        assert!(canvas.pointer_move(Point { x: 25.0, y: 25.0 }));
        assert!(canvas.pointer_move(Point { x: 31.25, y: 29.5 }));
        assert_eq!(
            canvas
                .group_preview
                .iter()
                .map(|(_, s)| s.points.as_ptr())
                .collect::<Vec<_>>(),
            pointers
        );
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(render(&canvas, false), committed_before);
        let preview = render(&canvas, true);
        assert!(canvas.pointer_up(Point { x: 31.25, y: 29.5 }));
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(canvas.selected_indices(), vec![0, 1]);
        assert_eq!(preview, render(&canvas, false), "{tool:?}");
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas
                .editor
                .elements()
                .iter()
                .map(|s| s.points.clone())
                .collect::<Vec<_>>(),
            original
        );
    }
}

#[test]
fn effect_drag_reuses_geometry_and_matches_committed_pixels() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Blur, Tool::Mosaic] {
        let mut canvas = canvas(tool);
        let original = canvas.editor.elements()[0].clone();
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(Point { x: 24.0, y: 24.0 }));
        let allocation = canvas
            .selection_effect_preview
            .as_ref()
            .unwrap()
            .points
            .as_ptr();
        for point in [Point { x: 30.0, y: 26.0 }, Point { x: 31.25, y: 29.5 }] {
            assert!(canvas.pointer_move(point));
            assert_eq!(
                canvas
                    .selection_effect_preview
                    .as_ref()
                    .unwrap()
                    .points
                    .as_ptr(),
                allocation
            );
            assert_eq!(canvas.editor.revision(), revision);
            assert_eq!(canvas.editor.elements()[0].points, original.points);
        }
        if tool == Tool::Blur {
            assert!(canvas
                .selection_effect_preview
                .as_ref()
                .unwrap()
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(original.blur.as_ref().unwrap().pixels()));
        }
        let points = canvas
            .selection_effect_preview
            .as_ref()
            .unwrap()
            .points
            .clone();
        let preview = render(&canvas, true);
        assert!(preview.iter().any(|byte| *byte != 0));
        assert!(canvas.pointer_up(Point { x: 31.25, y: 29.5 }));
        assert!(canvas.selection_effect_preview.is_none());
        assert_eq!(canvas.editor.elements()[0].points, points);
        assert_eq!(preview, render(&canvas, false), "{tool:?}");
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].points, original.points);
    }
}

#[test]
fn effect_drag_rejects_invalid_moves_and_releases_cancelled_or_stale_preview() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Blur, Tool::Mosaic] {
        let mut canvas = canvas(tool);
        let revision = canvas.editor.revision();
        let start = Point { x: 24.0, y: 24.0 };
        assert!(canvas.pointer_down(start));
        let points = canvas
            .selection_effect_preview
            .as_ref()
            .unwrap()
            .points
            .clone();
        for x in [f32::NAN, f32::INFINITY, 2_000_000.0] {
            assert!(!canvas.pointer_move(Point { x, y: 24.0 }));
            assert_eq!(
                canvas.selection_effect_preview.as_ref().unwrap().points,
                points
            );
        }
        assert!(canvas.cancel_gesture());
        assert!(canvas.selection_effect_preview.is_none());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.pointer_down(start));
        assert!(canvas.editor.undo());
        assert!(canvas.pointer_move(start));
        assert!(canvas.selection_effect_preview.is_none());
        assert!(canvas.selection_drag.is_none());
    }
}

#[test]
fn rotated_blur_mask_keeps_original_source_sampling_coordinates() {
    let _apartment = Apartment::new().unwrap();
    let mut canvas = canvas(Tool::Blur);
    assert!(canvas
        .editor
        .rotate_many_around(&[0], Point { x: 32.0, y: 32.0 }, 90.0)
        .unwrap());
    let output = render(&canvas, false);
    let index = (26 * 64 + 40) * 4;
    let source = &canvas.image.as_ref().unwrap().pixels;
    for channel in 0..4 {
        // 模糊呈现保留主干的 8% 白色覆盖，不能直接比较未覆盖的源像素。
        let expected = if channel == 3 {
            255.0
        } else {
            source[index + channel] as f32 * 0.92 + 255.0 * 0.08
        };
        assert!((output[index + channel] as f32 - expected).abs() <= 1.0);
    }
    assert_eq!(output[(24 * 64 + 24) * 4 + 3], 0);
}

#[test]
fn single_effect_rotation_matches_geometry_rotation_without_rotating_source() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Blur, Tool::Mosaic] {
        let mut canvas = canvas(tool);
        let original = canvas.editor.elements()[0].points.clone();
        let pivot = crate::rotation::center(&original);
        canvas.selected = Some((0, canvas.editor.revision()));
        assert!(canvas.rotate_selected(90.0));
        assert_eq!(canvas.editor.elements()[0].rotation, 0.0);
        let rendered = render(&canvas, false);
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].points, original);
        assert!(canvas.editor.rotate_many_around(&[0], pivot, 90.0).unwrap());
        assert_eq!(rendered, render(&canvas, false));
        if tool == Tool::Blur {
            let index = (30 * 64 + 26) * 4;
            let source = &canvas.image.as_ref().unwrap().pixels;
            for channel in 0..3 {
                let expected = source[index + channel] as f32 * 0.92 + 255.0 * 0.08;
                assert!((rendered[index + channel] as f32 - expected).abs() <= 1.0);
            }
            assert_eq!(rendered[index + 3], 255);
            // 原水平遮罩的左端在旋转后应消失。
            assert_eq!(rendered[(24 * 64 + 16) * 4 + 3], 0);
        }
    }
}

#[test]
fn rotation_handle_preview_pixels_match_release_and_cancel_has_no_history() {
    let _apartment = Apartment::new().unwrap();
    for tool in [Tool::Blur, Tool::Mosaic] {
        let mut canvas = canvas(tool);
        canvas.selected = Some((0, canvas.editor.revision()));
        let handle = super::super::rotation_handle::position(&canvas).unwrap();
        let pivot = crate::rotation::center(&canvas.editor.elements()[0].points);
        let end = Point {
            x: pivot.x + 20.0,
            y: pivot.y,
        };
        let revision = canvas.editor.revision();
        let original = canvas.editor.elements()[0].points.clone();
        assert!(canvas.pointer_down(handle));
        assert!(canvas.rotation_drag.is_some());
        let buffer = canvas.group_preview[0].1.points.as_ptr();
        assert!(canvas.pointer_move(end));
        assert_eq!(canvas.group_preview[0].1.points.as_ptr(), buffer);
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(canvas.editor.elements()[0].points, original);
        let preview = render(&canvas, true);
        assert!(canvas.pointer_up(end));
        assert!(canvas.rotation_drag.is_none());
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(preview, render(&canvas, false));
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.elements()[0].points, original);
        canvas.selected = Some((0, canvas.editor.revision()));
        assert!(canvas.pointer_down(handle));
        assert!(canvas.pointer_move(end));
        let revision = canvas.editor.revision();
        assert!(canvas.cancel_gesture());
        assert!(canvas.rotation_drag.is_none());
        assert!(canvas.group_preview.is_empty());
        assert_eq!(canvas.editor.revision(), revision);
    }
}

#[test]
fn scale_handle_mosaic_preview_matches_prepared_commit() {
    let _apartment = Apartment::new().unwrap();
    let mut canvas = canvas(Tool::Mosaic);
    canvas.selected = Some((0, canvas.editor.revision()));
    let handle = super::super::scale_handle::position(&canvas).unwrap();
    let pivot = crate::rotation::center(&canvas.editor.elements()[0].points);
    let end = Point {
        x: pivot.x + (handle.x - pivot.x) * 1.5,
        y: pivot.y + (handle.y - pivot.y) * 1.5,
    };
    let revision = canvas.editor.revision();
    assert!(canvas.pointer_down(handle));
    assert!(canvas.scale_drag.is_some());
    let pointer = canvas.group_preview[0].1.points.as_ptr();
    assert!(canvas.pointer_move_modified(end, true, false));
    assert_eq!(canvas.group_preview[0].1.points.as_ptr(), pointer);
    let preview = render(&canvas, true);
    assert!(canvas.pointer_up_modified(end, true, false));
    assert_eq!(canvas.editor.revision(), revision);
    let plan = canvas.scale_pending.take().unwrap();
    assert!(canvas.editor.commit_resize(plan).unwrap());
    assert_eq!(preview, render(&canvas, false));
    assert_eq!(canvas.editor.revision(), revision + 1);
}

#[test]
fn blur_scale_release_retains_preview_through_capture_loss_until_handoff() {
    let _apartment = Apartment::new().unwrap();
    let mut canvas = canvas(Tool::Blur);
    canvas.selected = Some((0, canvas.editor.revision()));
    let committed = render(&canvas, false);
    let handle = super::super::scale_handle::position(&canvas).unwrap();
    let pivot = crate::rotation::center(&canvas.editor.elements()[0].points);
    let end = Point {
        x: pivot.x + (handle.x - pivot.x) * 1.5,
        y: pivot.y + (handle.y - pivot.y) * 1.5,
    };
    assert!(canvas.pointer_down(handle));
    assert!(canvas.pointer_move(end));
    let preview = render(&canvas, true);
    assert!(canvas.pointer_up(end));
    let plan = canvas.scale_pending.take().unwrap();
    assert!(canvas.scale_display_revision.is_some());
    let pointer = canvas.group_preview[0].1.points.as_ptr();
    // 宿主先接管候选，再 ReleaseCapture；捕获丢失不能清除待交付显示。
    canvas.cancel_gesture();
    canvas.validate_scale_display();
    assert_eq!(canvas.group_preview[0].1.points.as_ptr(), pointer);
    assert_eq!(preview, render(&canvas, true));
    assert_eq!(committed, render(&canvas, false));
    let mut job = crate::resize_job::Job::default();
    job.start(plan).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let plan = loop {
        if let Some(result) = job.poll() {
            break result.unwrap();
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(canvas.clear_scale_display());
    assert!(canvas.editor.commit_resize(plan).unwrap());
    assert!(canvas.group_preview.is_empty());
    assert_eq!(
        canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        12.0
    );
    assert!(canvas.editor.undo());
    assert_eq!(committed, render(&canvas, false));
}
