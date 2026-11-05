//! 双层擦除必须先逐层衰减 alpha，再 source-over；合成后统一遮罩不是同一个操作。
use super::*;
use windows::Win32::{Graphics::Imaging::*, System::Com::*};

#[test]
fn content_preview_blends_individually_erased_layers_at_soft_edges() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(32, 32, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
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
        let image = Image {
            width: 32,
            height: 32,
            pixels: vec![255; 32 * 32 * 4].into(),
        };
        let request = crate::erase_job::Request {
            preview_identity: std::sync::Arc::new(()),
            annotation_frame: None,
            base_projection: None,
            source: Layers {
                annotation_size: None,
                size: (32, 32),
                base: image.pixels.clone(),
                annotations: Some(image.pixels.clone()),
            },
            revision: 0,
            mode: crate::erase_layers::Mode::Content,
            points: vec![Point {
                x: 16.25,
                y: 16.375,
            }],
            width: 9.25,
        };
        let mut resources = Resources::default();
        let mut render = |overlay: Option<&Pixels>| {
            resources.sync(&target, Some(&image), overlay).unwrap();
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            resources
                .draw_erasing(
                    &target,
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: 32.0,
                        bottom: 32.0,
                    },
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: 32.0,
                        bottom: 32.0,
                    },
                    Some(&request),
                    Viewport {
                        origin: Point::default(),
                        scale: 1.0,
                    },
                )
                .unwrap();
            target.EndDraw(None, None).unwrap();
            let mut bytes = vec![0; 32 * 32 * 4];
            output
                .CopyPixels(std::ptr::null(), 32 * 4, &mut bytes)
                .unwrap();
            bytes
        };
        let single = render(None);
        let double = render(Some(&image.pixels));
        let mut soft = 0;
        for (i, (&one, &two)) in single.iter().zip(&double).enumerate() {
            let expected = one as f32 + one as f32 * (1.0 - one as f32 / 255.0);
            if one > 0 && one < 255 {
                soft += 1;
            }
            assert!(
                (two as f32 - expected).abs() <= 1.0,
                "byte={i} single={one} double={two} expected={expected}"
            );
        }
        assert!(soft > 0, "fixture must exercise antialiased boundary");
    }
}
