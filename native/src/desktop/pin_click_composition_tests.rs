//! Test only owned no-activate blocks, without moving the cursor or injecting
//! input. Desktop pixels and WindowFromPoint are necessary, not sufficient,
//! conditions for passing the separate physical input delivery gate.
use super::*;
use crate::desktop::pin_click_through::Styles;

#[test]
#[ignore = "visible DWM pixel probe; requires an interactive desktop"]
fn d3d11_pin_style_keeps_desktop_pixels_and_redirects_hit() {
    probe(11);
}
#[test]
#[ignore = "visible DWM pixel probe; requires an interactive desktop"]
fn d3d12_pin_style_keeps_desktop_pixels_and_redirects_hit() {
    probe(12);
}
#[test]
#[ignore = "visible DWM pixel probe; requires an interactive desktop"]
fn software_pin_style_keeps_desktop_pixels_and_redirects_hit() {
    probe(0);
}
fn probe(backend: u8) {
    let _serial = crate::desktop::composition_retirement::serial_for_test();
    let _apt = Apartment::new().unwrap();
    unsafe {
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!previous.0.is_null());
        let _dpi = DpiScope(previous);
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let blue = CreateSolidBrush(COLORREF(0xff0000));
        assert!(!blue.is_invalid());
        if RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Software.Transition.Test"),
            lpfnWndProc: Some(procedure),
            hbrBackground: blue,
            ..Default::default()
        }) == 0
        {
            let _ = DeleteObject(blue.into());
            panic!("cannot register probe window");
        }
        let _class = Class(instance);
        let back = window(instance, false, 32, 128);
        show(back.0);
        expect_pixel(back.0, 32, [0, 0, 255]);
        let front = window(instance, true, 48, 64);
        let size = D2D_SIZE_U {
            width: 64,
            height: 64,
        };
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let paint = |target: &ID2D1RenderTarget| {
            target.Clear(Some(&D2D1_COLOR_F {
                r: 1.,
                g: 0.,
                b: 0.,
                a: 1.,
            }));
            Ok(())
        };
        let (mut gpu, mut cpu) = (None, None);
        if backend == 0 {
            let style = Transition::begin(front.0, true).unwrap();
            let mut surface = SoftwareSurface::new(&factory, size).unwrap();
            surface.draw(paint).unwrap();
            surface.present(front.0, true, 1.).unwrap();
            style.commit().unwrap();
            cpu = Some(surface);
        } else {
            let mut surface = if backend == 11 {
                compatibility(front.0, size)
            } else {
                RenderSurface::D3d12(Box::new(CompositionD3d12::new(front.0, size).unwrap()))
            };
            assert!(surface.draw(paint).unwrap());
            gpu = Some(surface);
        }
        let region = CreateRectRgn(0, 0, 56, 56);
        assert_ne!(SetWindowRgn(front.0, Some(region), false), 0);
        show(front.0);
        let p = POINT { x: 64, y: 64 };
        expect_pixel(front.0, 16, [255, 0, 0]);
        assert_eq!(WindowFromPoint(p), front.0);
        let owns = || IsWindow(Some(front.0)).as_bool();
        let style = Styles::snapshot(front.0, &owns).unwrap();
        Styles::enable(front.0, &owns).unwrap();
        assert_ne!(SetWindowRgn(front.0, None, false), 0);
        expect_pixel(front.0, 16, [255, 0, 0]);
        assert_eq!(WindowFromPoint(p), back.0);
        style.restore(front.0, &owns).unwrap();
        let restored = CreateRectRgn(0, 0, 56, 56);
        assert_ne!(SetWindowRgn(front.0, Some(restored), false), 0);
        assert_eq!(WindowFromPoint(p), front.0);
        expect_pixel(front.0, 16, [255, 0, 0]);
        drop((gpu, cpu));
    }
}
