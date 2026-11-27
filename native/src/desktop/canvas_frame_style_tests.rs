//! Pixel regressions for production Canvas and complete software snapshots; no physical-desktop capture or user-renderer switching.
use super::*;
use crate::{desktop::presentation_readback_test::Pixels, pin_frame_style::Style};
fn style() -> Style {
    Style::read(&serde_json::json!({"borderWidth":2,"borderColor":"red","cornerRadius":5})).unwrap()
}
fn at(p: &Pixels, x: usize, y: usize) -> &[u8] {
    &p.bytes[(y * p.size[0] as usize + x) * 4..(y * p.size[0] as usize + x + 1) * 4]
}
fn read(c: &mut Canvas) -> Pixels {
    // Read only this test's offscreen frame: return the submission lease first, without submitting pixels to the physical desktop.
    drop(c.take_submission(false).unwrap());
    c.target.as_ref().unwrap().surface.readback_for_test()
}
#[test]
fn unresolved_frame_does_not_paint_retained_or_fallback_pixels() {
    software_tests::fixture_mode(true, |state, hwnd| {
        let mut c = state.canvas.borrow_mut();
        let external = crate::desktop::display_frame::Frame::capture(
            state.unit_id.clone(),
            c.image.clone().unwrap(),
            None,
        );
        c.display_waiting = true;
        for frame in [Some(&external), None] {
            c.paint_display(hwnd, 40, 40, 96, frame).unwrap();
            let p = read(&mut c);
            let first = &p.bytes[..4];
            assert_eq!(first[3], 255);
            assert_eq!(first[0], first[2]);
            assert!(p.bytes.chunks_exact(4).all(|px| px == first));
        }
    });
}
#[test]
fn normal_external_and_zoom_snapshot_keep_same_style_and_fixed_dip_width() {
    software_tests::fixture_mode(true, |state, hwnd| {
        let mut c = state.canvas.borrow_mut();
        c.frame_style = style();
        c.viewport = Viewport {
            origin: Point { x: 0., y: 0. },
            scale: 5.,
        };
        let stamp = c.editor.document_stamp();
        for (size, dpi) in [(40, 96), (60, 144), (80, 192)] {
            c.paint(hwnd, size, size, dpi).unwrap();
            let p = read(&mut c);
            assert_eq!(at(&p, 0, 0)[3], 0);
            assert_eq!(at(&p, (size / 2) as usize, 1), &[0, 0, 255, 255]);
            assert_eq!(
                at(&p, (size / 2) as usize, (size / 2) as usize),
                &[30, 60, 90, 255]
            );
        }
        let source = c.image.clone().unwrap();
        let frame =
            crate::desktop::display_frame::Frame::capture(state.unit_id.clone(), source, None);
        c.paint_display(hwnd, 40, 40, 96, Some(&frame)).unwrap();
        let external = read(&mut c);
        assert_eq!(at(&external, 20, 1), &[0, 0, 255, 255]);
        let mut copy = c.zoom_snapshot(Some(&frame)).unwrap().unwrap();
        assert_eq!(copy.frame_style, c.frame_style);
        // The production zoom entry point continues passing the resolved Frame; external pixels do not masquerade as a local raster document.
        copy.paint_display(hwnd, 40, 40, 96, Some(&frame)).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*external.bytes);
        assert!(stamp.matches(&c.editor));
    });
}
#[test]
fn minified_frame_styles_cover_entire_small_window_not_the_expanded_source_frame() {
    software_tests::fixture_mode(true, |state, hwnd| {
        let mut c = state.canvas.borrow_mut();
        c.frame_style = style();
        let view = crate::session_minified::View {
            size: crate::crop_geometry::Size {
                width: 8.,
                height: 8.,
            },
            offset: (0., 0.),
        };
        c.minified = Some(
            minified::Presentation::restored(
                c.image.clone(),
                crate::session_minified::Views {
                    image: view,
                    annotations: view,
                },
                view.size,
                &c.editor,
            )
            .unwrap(),
        );
        c.paint(hwnd, 40, 40, 96).unwrap();
        let p = read(&mut c);
        assert_eq!(at(&p, 0, 0)[3], 0);
        assert_eq!(at(&p, 20, 1), &[0, 0, 255, 255]);
        assert_eq!(at(&p, 20, 3), &[30, 60, 90, 255]);
        let mut copy = c.zoom_snapshot(None).unwrap().unwrap();
        copy.paint(hwnd, 40, 40, 96).unwrap();
        assert_eq!(&*read(&mut copy).bytes, &*p.bytes);
    });
}
#[test]
fn content_border_is_visible_in_contain_letterbox_and_not_cut_to_base_bitmap() {
    software_tests::fixture_mode(true, |state, hwnd| {
        let mut c = state.canvas.borrow_mut();
        c.frame_style = style();
        c.viewport = Viewport {
            origin: Point { x: 10., y: 10. },
            scale: 2.5,
        };
        c.paint(hwnd, 40, 40, 96).unwrap();
        let p = read(&mut c);
        assert_eq!(at(&p, 20, 1), &[0, 0, 255, 255]);
        assert_eq!(at(&p, 20, 5)[3], 0);
        assert_eq!(at(&p, 20, 20), &[30, 60, 90, 255]);
    });
}

#[test]
fn external_magnifier_does_not_sample_unpinned_workspace_background() {
    software_tests::fixture_mode(false, |state, hwnd| {
        let image = Image {
            width: 40,
            height: 40,
            pixels: (0..1600)
                .flat_map(|i| if i % 40 < 20 { [0; 4] } else { [255; 4] })
                .collect::<Vec<_>>()
                .into(),
        };
        let mut editor = Editor::default();
        editor
            .begin(Tool::Magnifier, Point { x: 20., y: 10. })
            .unwrap();
        editor.extend(Point { x: 40., y: 30. }).unwrap();
        assert!(editor.commit());
        editor
            .move_lens_handle(0, 0, Point { x: 5., y: 20. })
            .unwrap();
        let frame = crate::desktop::display_frame::Frame::capture(
            state.unit_id.clone(),
            image.clone(),
            Some(&editor),
        );
        let output = crate::desktop::export::decode_png(
            crate::desktop::export::style::Snapshot::default()
                .encoded_png(&image, &editor, 1.)
                .unwrap()
                .to_vec(),
        );
        let mut c = state.canvas.borrow_mut();
        c.viewport = Viewport {
            origin: Point::default(),
            scale: 1.,
        };
        c.paint_display(hwnd, 40, 40, 96, Some(&frame)).unwrap();
        let pixels = read(&mut c);
        assert_eq!(at(&pixels, 30, 20), &[255; 4]);
        assert_eq!(
            at(&pixels, 30, 20),
            &output.pixels[(20 * 40 + 30) * 4..(20 * 40 + 31) * 4]
        );
    });
}
