//! 生产软件 Canvas 的离屏像素：扩展覆盖层不进入底图/导出模型，缩放复用同一视图。
use super::*;
use crate::desktop::extension_visuals::{clear, Views};
use crate::desktop::presentation_readback_test::Pixels;
fn read(c: &mut Canvas) -> Pixels {
    drop(c.take_submission(false).unwrap());
    c.target.as_ref().unwrap().surface.readback_for_test()
}
fn image() -> Image {
    Image {
        width: 64,
        height: 64,
        pixels: vec![255; 64 * 64 * 4].into(),
    }
}
#[test]
fn production_canvas_and_zoom_snapshot_share_clipped_overlay_without_baking_source() {
    for trust in ["trusted", "unsigned_developer"] {
        software_tests::fixture_mode(true, |_, hwnd| {
            let mut c = Canvas::new(Some(image())).unwrap();
            c.pinned = true;
            c.viewport = Viewport {
                origin: Point::default(),
                scale: 1.,
            };
            c.software_renderer = true;
            let mut data = crate::extension_visual::tests::catalog_value();
            data["plugins"][0]["trustStatus"] = trust.into();
            let catalog = crate::loom_extension_snapshot::Snapshot::parse(
                &serde_json::to_vec(&data).unwrap(),
            )
            .unwrap();
            let store = crate::extension_visual::tests::attachments();
            c.extension_views = Some(
                Views::prepare(
                    crate::extension_visual::project(Some(&catalog), &store, 4),
                    [64.; 2],
                )
                .unwrap()
                .bind(&c, None),
            );
            c.paint(hwnd, 64, 64, 96).unwrap();
            let pixels = read(&mut c);
            let at = |x: usize, y: usize| &pixels.bytes[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
            assert_eq!(at(5, 7), &[0, 0, 255, 255]);
            assert_eq!(at(3, 7), &[255; 4]);
            assert!(c.image.as_ref().unwrap().pixels.iter().all(|v| *v == 255));
            assert!(c.editor.record_pin_change(
                crate::editor::PinValue::Opacity {
                    mini: false,
                    value: 1.
                },
                crate::editor::PinValue::Opacity {
                    mini: false,
                    value: 0.5
                },
            ));
            let mut copy = c.zoom_snapshot(None).unwrap().unwrap();
            copy.paint(hwnd, 64, 64, 96).unwrap();
            assert_eq!(&*read(&mut copy).bytes, &*pixels.bytes);
            clear(&mut c);
            copy.paint(hwnd, 64, 64, 96).unwrap();
            assert!(read(&mut copy).bytes.iter().all(|v| *v == 255));
        });
    }
}
#[test]
fn empty_auto_height_scene_does_not_fill_its_overlay_bounds() {
    software_tests::fixture_mode(true, |_, hwnd| {
        let mut c = Canvas::new(Some(image())).unwrap();
        c.pinned = true;
        c.software_renderer = true;
        c.viewport = Viewport {
            origin: Point::default(),
            scale: 1.,
        };
        let mut visual = crate::extension_visual::project(
            Some(&crate::extension_visual::tests::catalog()),
            &crate::extension_visual::tests::attachments(),
            4,
        );
        visual[0].scene.as_object_mut().unwrap().remove("layout");
        c.extension_views = Some(Views::prepare(visual, [64.; 2]).unwrap().bind(&c, None));
        c.paint(hwnd, 64, 64, 96).unwrap();
        assert!(read(&mut c).bytes.iter().all(|v| *v == 255));
    });
}
#[test]
fn external_frame_guard_rejects_changed_source_and_rebinds_zoom_without_baking_overlay() {
    software_tests::fixture_mode(true, |state, hwnd| {
        let mut c = Canvas::new(Some(image())).unwrap();
        c.pinned = true;
        c.viewport = Viewport {
            origin: Point::default(),
            scale: 1.,
        };
        c.software_renderer = true;
        let frame =
            crate::desktop::display_frame::Frame::capture(state.unit_id.clone(), image(), None);
        let replacement =
            crate::desktop::display_frame::Frame::capture(state.unit_id.clone(), image(), None);
        c.extension_views = Some(
            Views::prepare(
                crate::extension_visual::project(
                    Some(&crate::extension_visual::tests::catalog()),
                    &crate::extension_visual::tests::attachments(),
                    4,
                ),
                [64.; 2],
            )
            .unwrap()
            .bind(&c, Some(&frame)),
        );
        c.paint_display(hwnd, 64, 64, 96, Some(&frame)).unwrap();
        let original = read(&mut c);
        assert_eq!(
            &original.bytes[(7 * 64 + 5) * 4..(7 * 64 + 6) * 4],
            &[0, 0, 255, 255]
        );
        assert!(frame.image.pixels.iter().all(|v| *v == 255));
        let mut copy = c.zoom_snapshot(Some(&frame)).unwrap().unwrap();
        copy.paint_display(hwnd, 64, 64, 96, Some(&frame)).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*original.bytes);
        // 换源后的同步 paint 可能早于 Application 的目录结算，不能画上一来源的场景。
        for canvas in [&mut c, &mut copy] {
            canvas
                .paint_display(hwnd, 64, 64, 96, Some(&replacement))
                .unwrap();
            assert!(read(canvas).bytes.iter().all(|v| *v == 255));
        }
        assert!(c
            .zoom_snapshot(Some(&replacement))
            .unwrap()
            .unwrap()
            .extension_views
            .is_none());
        clear(&mut c);
        copy.paint_display(hwnd, 64, 64, 96, Some(&frame)).unwrap();
        assert!(read(&mut copy).bytes.iter().all(|v| *v == 255));
    });
}
#[test]
fn selection_highlight_updates_the_same_canvas_and_shared_complete_zoom_snapshot() {
    software_tests::fixture_mode(true, |_, hwnd| {
        let mut c = Canvas::new(Some(image())).unwrap();
        c.pinned = true;
        c.software_renderer = true;
        c.viewport = Viewport {
            origin: Point::default(),
            scale: 1.,
        };
        let mut visuals = crate::extension_visual::project(
            Some(&crate::extension_visual::tests::catalog()),
            &crate::extension_visual::tests::attachments(),
            4,
        );
        visuals[0].scene = serde_json::json!({"id":"text","type":"text","props":{"text":"Native","selectable":true},
            "layout":{"width":32,"height":20}});
        c.extension_views = Some(Views::prepare(visuals, [64.; 2]).unwrap().bind(&c, None));
        let views = c.extension_views.as_ref().unwrap().shared();
        c.paint(hwnd, 64, 64, 96).unwrap();
        let before = read(&mut c);
        let r = views.rects()[0];
        let point = [r.left + 0.1, r.top + 2.];
        views.begin(point, false).unwrap();
        views.finish([100., point[1]], Default::default()).unwrap();
        c.paint(hwnd, 64, 64, 96).unwrap();
        let selected = read(&mut c);
        assert_ne!(&*selected.bytes, &*before.bytes);
        let mut copy = c.zoom_snapshot(None).unwrap().unwrap();
        copy.paint(hwnd, 64, 64, 96).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*selected.bytes);
        views.cancel(true);
        copy.paint(hwnd, 64, 64, 96).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*before.bytes);
    });
}
#[test]
fn offline_badge_uses_production_canvas_and_shared_zoom_without_activating_ocr() {
    software_tests::fixture_mode(true, |_, hwnd| {
        let mut c = Canvas::new(Some(image())).unwrap();
        c.pinned = true;
        c.software_renderer = true;
        c.viewport = Viewport {
            origin: Point::default(),
            scale: 1.,
        };
        let store = crate::extension_visual::availability_tests::text_store();
        c.paint(hwnd, 96, 64, 96).unwrap();
        let baseline = read(&mut c);
        c.extension_views = Some(
            Views::prepare(crate::extension_visual::project(None, &store, 4), [64.; 2])
                .unwrap()
                .bind(&c, None),
        );
        assert!(!c.extension_views.as_ref().unwrap().active(&c, None, false));
        c.paint(hwnd, 64, 64, 96).unwrap();
        let first = read(&mut c);
        assert_ne!(
            &first.bytes[(27 * 64 + 10) * 4..(27 * 64 + 11) * 4],
            &[255; 4]
        );
        assert_eq!(
            &first.bytes[(27 * 64 + 61) * 4..(27 * 64 + 62) * 4],
            &[255; 4]
        );
        let mut copy = c.zoom_snapshot(None).unwrap().unwrap();
        copy.paint(hwnd, 96, 64, 96).unwrap();
        let resized = read(&mut copy);
        assert_ne!(
            &resized.bytes[(27 * 96 + 70) * 4..(27 * 96 + 71) * 4],
            &baseline.bytes[(27 * 96 + 70) * 4..(27 * 96 + 71) * 4]
        );
        c.paint(hwnd, 96, 64, 96).unwrap();
        assert_eq!(&*read(&mut c).bytes, &*resized.bytes);
        clear(&mut c);
        copy.paint(hwnd, 96, 64, 96).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*baseline.bytes);
    });
}
