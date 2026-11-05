//! 标记实际D2D/WIC独立颜色oracle；不读取桌面，不用本renderer计算期望像素。
use super::*;
use windows::Win32::{
    Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
    System::Com::*,
};
#[test]
fn real_marker_has_opaque_background_close_button_and_bounded_literal_identity_text() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(
                640,
                480,
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
                    r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.,
                    dpiY: 96.,
                    ..Default::default()
                },
            )
            .unwrap();
        let mut endpoint =
            crate::tile_wire::Endpoint::image("device-1", &crate::tile_tests::output());
        endpoint.display.as_mut().unwrap().name = "屏幕\n伪造\u{202e}".into();
        let now = Instant::now();
        let marker = Marker {
            identity: crate::tile_identification::Identity {
                endpoint: endpoint.endpoint_id.clone(),
                lease: "lease".into(),
                request: "request".into(),
            },
            endpoint,
            until: now + std::time::Duration::from_millis(1001),
        };
        let text = text(&marker, now);
        assert!(text.contains("2 秒后关闭"));
        assert!(text.contains("屏幕\\u{a}伪造\\u{202e}"));
        assert!(text.contains("设备：device-1") && text.contains("输出：monitor-"));
        let painter = Painter::new().unwrap();
        target.BeginDraw();
        painter.draw(&target, &marker, now).unwrap();
        target.EndDraw(None, None).unwrap();
        let mut pixels = vec![0; 640 * 480 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 640 * 4, &mut pixels)
            .unwrap();
        let pixel = |x: usize, y: usize| &pixels[(y * 640 + x) * 4..(y * 640 + x + 1) * 4];
        // 颜色浮点→8位允许1LSB；矩形内部不经过字体抗锯齿。
        for (actual, expected) in pixel(10, 10).iter().zip([15u8, 13, 10, 255]) {
            assert!(actual.abs_diff(expected) <= 1);
        }
        for (actual, expected) in pixel(560, 440).iter().zip([77u8, 230, 217, 255]) {
            assert!(actual.abs_diff(expected) <= 1);
        }
        assert!(pixels.chunks_exact(4).all(|p| p[3] == 255));
        drop(target);
        drop(bitmap);
        drop(wic);
        drop(factory);
        drop(painter);
        CoUninitialize();
    }
}
