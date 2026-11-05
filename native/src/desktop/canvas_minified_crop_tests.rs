//! 裁剪源坐标、独立栅格标注及预计算效果纹理经缩略变换后的真实像素验证。
use super::*;
use crate::{
    crop_geometry::{Crop, Rect},
    erase_layers::Layers,
};
use windows::Win32::{
    Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
    System::Com::*,
};

#[test]
fn cropped_external_effect_and_overlay_share_document_mapping_at_multiple_dpis() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 4,
        pixels: [100, 0, 0, 255].repeat(32).into(),
    };
    let mut editor = Editor::with_layers(Layers {
        size: (8, 4),
        base: image.pixels.clone(),
        annotations: Some([0, 180, 0, 255].repeat(4).into()),
        annotation_size: Some((2, 2)),
    })
    .unwrap();
    editor.set_style(
        crate::stroke_style::StrokeStyle::new(1.0, crate::stroke_style::Color(255, 0, 0)).unwrap(),
    );
    // 用红色标记预计算效果纹理，区分蓝色底图与绿色栅格标注；本测试不验证卷积算法。
    let effect =
        crate::blur_image::BlurImage::new(8, 4, [0, 0, 200, 255].repeat(32).into(), 2).unwrap();
    editor
        .append_restored_blur(vec![Point { x: 5.0, y: 2.0 }], 0.0, effect)
        .unwrap();
    let editor = editor
        .with_restored_crop(Crop {
            rect: Rect {
                x: 4.0,
                y: 1.0,
                width: 2.0,
                height: 2.0,
            },
            source: Size {
                width: 8.0,
                height: 4.0,
            },
        })
        .unwrap();
    let stamp = editor.document_stamp();
    let view = View {
        size: Size {
            width: 4.0,
            height: 4.0,
        },
        offset: (1.0, 1.0),
    };
    let local = Editor::default();
    let original = Presentation::restored(
        None,
        Views {
            image: view,
            annotations: view,
        },
        Size {
            width: 2.0,
            height: 2.0,
        },
        &local,
    )
    .unwrap();
    let external = original.for_display(&image, &editor).unwrap();
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let output = wic
            .CreateBitmap(8, 8, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let brush = target
            .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
            .unwrap();
        let mut resources = raster::Resources::default();
        let mut effects = Default::default();
        for dpi in [96, 144, 192] {
            external
                .draw(
                    &target,
                    &mut resources,
                    &brush,
                    &mut effects,
                    &editor,
                    (
                        D2D_SIZE_U {
                            width: 8,
                            height: 8,
                        },
                        dpi,
                    ),
                )
                .unwrap();
            let mut pixels = vec![0; 256];
            output
                .CopyPixels(std::ptr::null(), 32, &mut pixels)
                .unwrap();
            assert_eq!(&pixels[..4], &[0, 180, 0, 255]);
            let center = &pixels[(4 * 8 + 4) * 4..(4 * 8 + 5) * 4];
            assert!(
                center[0] < 30 && center[1] < 30 && center[2] >= 195 && center[3] == 255,
                "Effect did not follow cropped view at DPI {dpi}: {center:?}"
            );
        }
    }
    assert!(stamp.matches(&editor));
    assert!(local.layers().is_none());
    assert!(original.matches(&local));
}
