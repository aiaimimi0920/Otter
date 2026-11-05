//! 同一 WIC 目标比较真实帧与独立 DirectWrite 省略参考，避免仅断言 CSS 字符串/metrics。
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
fn text_overflow_pixels_draw_ellipsis_not_clip_and_keep_content_clip() {
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let _apartment = Apartment;
    let text = "MMMMMMMMMMMM";
    let clipped = frame(scene("clip", 84., 32., text));
    let ellipsis = frame(scene("ellipsis", 84., 32., text));
    let engine = Engine::new().unwrap();
    let reference = engine
        .layout(
            text,
            Style {
                whitespace: Whitespace::Nowrap,
                ..style()
            },
            84.,
            32.,
        )
        .unwrap();
    unsafe {
        let sign = engine
            .factory
            .CreateEllipsisTrimmingSign(&reference.native)
            .unwrap();
        reference
            .native
            .SetTrimming(
                &DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    ..Default::default()
                },
                &sign,
            )
            .unwrap();
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
        let brush = target
            .CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.,
                    g: 1.,
                    b: 1.,
                    a: 1.,
                },
                None,
            )
            .unwrap();
        let render = |draw: &dyn Fn()| {
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            draw();
            target.EndDraw(None, None).unwrap();
            let mut bytes = vec![0; 160 * 64 * 4];
            bitmap
                .CopyPixels(std::ptr::null(), 160 * 4, &mut bytes)
                .unwrap();
            bytes
        };
        let a = render(&|| clipped.text().draw(&target, &brush).unwrap());
        let b = render(&|| ellipsis.text().draw(&target, &brush).unwrap());
        let expected = render(&|| reference.draw(&target, &brush, [0., 0.]).unwrap());
        assert_ne!(a, b, "CSS ellipsis must change actual glyph pixels");
        assert_eq!(
            b, expected,
            "final frame must render the native trimming sign"
        );
        assert!(b.chunks_exact(4).any(|pixel| pixel[3] > 0));
        assert!(b
            .chunks_exact(4)
            .enumerate()
            .all(|(i, p)| (i % 160 < 84 && i / 160 < 32) || p[3] == 0));
        // 小于省略号的内容盒仍严格裁剪，不能越界画出整个 sign。
        let tiny = frame(scene("ellipsis", 1., 32., text));
        let bytes = render(&|| tiny.text().draw(&target, &brush).unwrap());
        assert!(bytes
            .chunks_exact(4)
            .enumerate()
            .all(|(i, p)| i % 160 < 1 || p[3] == 0));
    }
}
