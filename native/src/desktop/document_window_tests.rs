//! Very long screenshots scale window presentation only; owned hidden HWNDs verify original images, source coordinates, and edit dimensions are preserved.
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;

#[test]
fn capture_window_size_respects_surface_budget_without_changing_normal_import_scale() {
    for (width, height, expected) in [
        (320, 240, (320, 240)),
        (4096, 4096, (4096, 4096)),
        (4097, 4097, (4096, 4096)),
        (16384, 2, (8192, 1)),
        (2, 16384, (1, 8192)),
        (1, 16384, (1, 8192)),
        (16384, 1, (8192, 1)),
        (8192, 4096, (5792, 2896)),
    ] {
        let size = initial_size(width as f32, height as f32, true);
        assert_eq!(size, expected);
        assert!(size.0 <= 8192 && size.1 <= 8192);
        assert!(i64::from(size.0) * i64::from(size.1) <= 16_777_216);
    }
    assert_eq!(initial_size(2000.0, 1000.0, false), (1200, 600));
    assert_eq!(initial_size(320.0, 240.0, false), (320, 240));
}

#[test]
fn long_capture_creates_a_bounded_window_and_keeps_full_source_pixels() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap();
    let _com = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let instance = crate::desktop::workspace_window_fixture::register();
        for (width, height) in [(2, 16384), (16384, 2), (1, 16384), (16384, 1)] {
            let origin = Some(POINT { x: -50, y: 20 });
            let pixels: crate::pixels::Pixels = vec![255; (width * height * 4) as usize].into();
            let captured = crate::capture_job::Captured {
                backend: "gdi",
                preview: crate::capture_color::SdrFrame {
                    width,
                    height,
                    pixels: pixels.clone(),
                },
                original_lease: None,
                original: None,
                reason: None,
                downgraded: false,
                display: None,
                diagnostic: None,
            };
            let mut context = crate::capture_job::Context::region(origin, 2, 2, "gdi");
            context.kind = crate::session_capture::Kind::Long;
            let prepared = crate::desktop::open_image::PreparedDocument::from_capture_with_cancel(
                captured,
                true,
                origin,
                Some(context),
                || false,
            )
            .unwrap();
            let document =
                DocumentWindow::create(instance, State::from_prepared(prepared).unwrap(), false)
                    .unwrap();
            let mut rect = RECT::default();
            GetClientRect(document.window.0, &mut rect).unwrap();
            assert_eq!(
                (rect.right, rect.bottom),
                ((width as i32 / 2).max(1), (height as i32 / 2).max(1))
            );
            GetWindowRect(document.window.0, &mut rect).unwrap();
            assert_eq!((rect.left, rect.top), (-50, 20));
            let canvas = document.state.canvas.borrow();
            let image = canvas.image.as_ref().unwrap();
            assert_eq!((image.width, image.height), (width, height));
            assert!(image.pixels.same_storage(&pixels));
            let metadata = canvas.capture_source().unwrap().metadata();
            assert_eq!(metadata["kind"], "long");
            assert_eq!(metadata["sourceRect"]["w"], 2.0);
            assert_eq!(metadata["sourceRect"]["x"], -50.0);
            let dpi = GetDpiForWindow(document.window.0).max(1) as f32;
            assert!((canvas.viewport.scale * dpi / 96.0 - 0.5).abs() < 0.0001);
        }
    }
}
