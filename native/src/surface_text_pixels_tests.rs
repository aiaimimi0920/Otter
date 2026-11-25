//! Verify text clipping through real WIC/Direct2D output without depending on exact font pixel appearance.
use super::*;
use windows::Win32::{
    Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        Imaging::*,
    },
    System::Com::*,
};

#[test]
fn text_pixels_obey_ancestor_clip_and_restore_target_clip_stack() {
    // Each test thread independently owns COM lifetime; release resources before CoUninitialize.
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let _apartment = Apartment;
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"clip","type":"column","layout":{"width":24,"height":40,"overflowX":"hidden"},"children":[{"id":"text","type":"text","props":{"text":"MMMMMMMM"},"style":{"fontSize":24,"whiteSpace":"nowrap"},"layout":{"width":120,"height":40}}]}]}"#).unwrap();
    let prepared =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [160., 64.])
            .unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(
                160,
                64,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapCacheOnLoad,
            )
            .unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let target = factory
            .CreateWicBitmapRenderTarget(
                &bitmap,
                &D2D1_RENDER_TARGET_PROPERTIES {
                    dpiX: 96.,
                    dpiY: 96.,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    ..Default::default()
                },
            )
            .unwrap();
        let white = D2D1_COLOR_F {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        };
        let brush = target.CreateSolidColorBrush(&white, None).unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        prepared.text().draw(&target, &brush).unwrap();
        target.EndDraw(None, None).unwrap();
        let mut pixels = vec![0; 160 * 64 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 160 * 4, &mut pixels)
            .unwrap();
        assert!(pixels
            .chunks_exact(4)
            .enumerate()
            .any(|(i, p)| i % 160 < 24 && p[3] > 0));
        assert!(pixels
            .chunks_exact(4)
            .enumerate()
            .all(|(i, p)| i % 160 < 24 || p[3] == 0));
        // Paint a marker outside text bounds; if draw leaves a clip stack behind, this pixel stays transparent.
        target.BeginDraw();
        target.FillRectangle(
            &D2D_RECT_F {
                left: 140.,
                top: 50.,
                right: 150.,
                bottom: 60.,
            },
            &brush,
        );
        target.EndDraw(None, None).unwrap();
        bitmap
            .CopyPixels(std::ptr::null(), 160 * 4, &mut pixels)
            .unwrap();
        assert_eq!(
            &pixels[(55 * 160 + 145) * 4..(55 * 160 + 146) * 4],
            &[255; 4]
        );
        // Positive control: the same Layout without ancestor clipping really has glyph pixels outside the boundary.
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        let item = &prepared.text().items()[0];
        item.layout().draw(&target, &brush, item.origin()).unwrap();
        target.EndDraw(None, None).unwrap();
        bitmap
            .CopyPixels(std::ptr::null(), 160 * 4, &mut pixels)
            .unwrap();
        assert!(pixels
            .chunks_exact(4)
            .enumerate()
            .any(|(i, p)| i % 160 >= 24 && p[3] > 0));
    }
}
