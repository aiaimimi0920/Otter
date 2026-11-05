//! 实际D2D/WIC的极窄裁剪、空区域和擦除预览，不用屏幕截图或新renderer生成预期。
use super::*;
use windows::Win32::{Graphics::Imaging::*, System::Com::*};
#[test]
fn narrow_legacy_draw_is_bounded_for_both_axes_and_empty_region_never_falls_back() {
    let _com = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        for vertical in [false, true] {
            let (w, h) = if vertical { (1, 8192) } else { (8192, 1) };
            let output = wic
                .CreateBitmap(w, h, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
                .unwrap();
            let target = factory
                .CreateWicBitmapRenderTarget(
                    &output,
                    &D2D1_RENDER_TARGET_PROPERTIES {
                        dpiX: 96.0,
                        dpiY: 96.0,
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        ..Default::default()
                    },
                )
                .unwrap();
            let mut bytes = [0, 200, 0, 255].repeat(8192);
            bytes[1234 * 4..1234 * 4 + 4].copy_from_slice(&[200, 0, 0, 255]);
            let image = Image {
                width: w,
                height: h,
                pixels: bytes.into(),
            };
            let source = crate::crop_geometry::Rect {
                x: if vertical { 0.0 } else { 1234.0 },
                y: if vertical { 1234.0 } else { 0.0 },
                width: 1.0,
                height: 1.0,
            };
            let full = crate::crop_geometry::Rect {
                x: 0.0,
                y: 0.0,
                width: w as f64,
                height: h as f64,
            };
            let rect = D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: w as f32,
                bottom: h as f32,
            };
            let mut resources = Resources::default();
            resources
                .sync_sized(&target, Some(&image), None, (0, 0))
                .unwrap();
            for empty in [false, true] {
                let mapping = crate::image_projection::Projection {
                    source: crate::crop_geometry::Rect {
                        x: if empty { 9000.0 } else { source.x },
                        ..source
                    },
                    target: full,
                };
                target.BeginDraw();
                target.Clear(Some(&D2D1_COLOR_F::default()));
                resources
                    .draw_erasing_projected(
                        &target,
                        &rect,
                        &rect,
                        None,
                        Viewport {
                            origin: Point::default(),
                            scale: 1.0,
                        },
                        Some(mapping),
                    )
                    .unwrap();
                target.EndDraw(None, None).unwrap();
                let mut actual = vec![0; 8192 * 4];
                output
                    .CopyPixels(std::ptr::null(), w * 4, &mut actual)
                    .unwrap();
                if empty {
                    assert!(actual.iter().all(|v| *v == 0));
                } else {
                    // Canvas沿用LINEAR：源矩形不是另建的裁图，边缘会采到相邻绿色。
                    // 两端各50%混色、中心纯蓝是独立解析oracle，不套工作流NEAREST的预期。
                    for (at, expected) in [
                        (0, [100, 100, 0, 255]),
                        (4095, [200, 0, 0, 255]),
                        (4096, [200, 0, 0, 255]),
                        (8191, [100, 100, 0, 255]),
                    ] {
                        assert_eq!(&actual[at * 4..at * 4 + 4], &expected, "D2D witness {at}");
                    }
                }
                let cpu = crate::raster_projection::project_region(
                    &image.pixels,
                    (w, h),
                    (w, h),
                    mapping,
                    || false,
                )
                .unwrap();
                if empty {
                    assert!(cpu.iter().all(|v| *v == 0));
                } else {
                    for (at, pixel) in cpu.chunks_exact(4).enumerate() {
                        let green =
                            (200.0 * ((at as f64 + 0.5) / 8192.0 - 0.5).abs()).round() as u8;
                        let blue = (200.0 * (1.0 - ((at as f64 + 0.5) / 8192.0 - 0.5).abs()))
                            .round() as u8;
                        assert_eq!(pixel, &[blue, green, 0, 255], "CPU oracle {at}");
                    }
                    for at in [0, 4095, 4096, 8191] {
                        assert_eq!(&cpu[at * 4..at * 4 + 4], &actual[at * 4..at * 4 + 4]);
                    }
                }
            }
            assert_eq!(&image.pixels[1234 * 4..1234 * 4 + 4], &[200, 0, 0, 255]);
        }
    }
}
