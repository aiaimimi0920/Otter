//! Real Canvas bitmap uploads and pixel readback on each selected native backend.
use super::*;

#[derive(Clone, Copy, Debug)]
enum Backend {
    Software,
    D3d11,
    D3d12,
}

fn draw_long_image(backend: Backend) {
    for vertical in [false, true] {
        software_tests::fixture_mode(true, |state, hwnd| {
            let mut canvas = state.canvas.borrow_mut();
            let (width, height) = if vertical { (8, 16384) } else { (16384, 8) };
            let colors = [[12, 42, 93, 255], [129, 82, 23, 255]];
            let mut pixels = vec![0; (width * height * 4) as usize];
            for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
                let axis = if vertical {
                    index / width as usize
                } else {
                    index % width as usize
                };
                pixel.copy_from_slice(&colors[usize::from(axis >= 16352)]);
            }
            canvas
                .replace_image(Image {
                    width,
                    height,
                    pixels: pixels.into(),
                })
                .unwrap();
            canvas.software_renderer = matches!(backend, Backend::Software);
            if matches!(backend, Backend::D3d12) {
                canvas.primary = crate::desktop::render_surface::Primary::D3d12;
            }
            canvas.compatibility_renderer = false;
            // A tiny target views the far end of the full-sized uploaded texture.
            let (w, h) = if vertical { (8, 64) } else { (64, 8) };
            canvas.viewport = Viewport {
                scale: 1.0,
                origin: if vertical {
                    Point {
                        x: 0.0,
                        y: -16320.0,
                    }
                } else {
                    Point {
                        x: -16320.0,
                        y: 0.0,
                    }
                },
            };
            canvas
                .paint(hwnd, w, h, 96)
                .unwrap_or_else(|error| panic!("{backend:?} {width}x{height}: {error}"));
            if matches!(backend, Backend::Software) {
                // Offscreen pixel proof, independent of desktop GDI submission.
                assert!(canvas.take_submission(true).is_some());
            } else {
                assert_eq!(
                    canvas.confirmed_paints, 1,
                    "{backend:?} did not confirm a GPU frame"
                );
            }
            let surface = &canvas.target.as_ref().unwrap().surface;
            assert!(matches!(
                (backend, surface),
                (Backend::Software, RenderSurface::Software(_))
                    | (Backend::D3d11, RenderSurface::D3d11(_))
                    | (Backend::D3d12, RenderSurface::D3d12(_))
            ));
            let readback = surface.readback_for_test();
            assert_eq!(readback.size, [w, h]);
            for (index, pixel) in readback.bytes.chunks_exact(4).enumerate() {
                let axis = if vertical {
                    index / w as usize
                } else {
                    index % w as usize
                };
                assert_eq!(
                    pixel,
                    &colors[usize::from(axis >= 32)],
                    "{backend:?} pixel {index}"
                );
            }
        });
    }
}

#[test]
fn long_raster_upload_software() {
    draw_long_image(Backend::Software);
}

#[test]
#[ignore = "requires a usable D3D11 composition backend on the active desktop"]
fn long_raster_upload_d3d11() {
    draw_long_image(Backend::D3d11);
}

#[test]
#[ignore = "requires a usable D3D12 composition backend on the active desktop"]
fn long_raster_upload_d3d12() {
    draw_long_image(Backend::D3d12);
}
