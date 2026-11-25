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
