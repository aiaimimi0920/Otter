//! WIC/D2D 像素夹具：统一 COM、画刷恢复与裁剪栈验证；不属于产品运行时。
use super::*;
use windows::Win32::{
    Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        Imaging::*,
    },
    System::Com::*,
};
pub(crate) fn render(
    json: &[u8],
    fixed_height: bool,
    visual: crate::surface_button_visual::State<'_>,
    prepare: impl FnOnce(&mut crate::surface_presentation::Presentation),
    switch_value: impl Fn(&str) -> Option<bool>,
) -> Vec<u8> {
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
    let _com = Apartment;
    // 绘制合同默认使用显式 64px 根高；自然根高度另用 fixed_height=false 验证。
    let mut json: serde_json::Value = serde_json::from_slice(json).unwrap();
    if fixed_height {
        let layout = json
            .as_object_mut()
            .unwrap()
            .entry("layout")
            .or_insert_with(|| serde_json::json!({}));
        layout
            .as_object_mut()
            .unwrap()
            .entry("height")
            .or_insert(serde_json::json!(64));
    }
    let scene = Scene::parse(&serde_json::to_vec(&json).unwrap()).unwrap();
    let mut presentation =
        crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
            .unwrap();
    prepare(&mut presentation);
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(64, 64, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
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
        let color = D2D1_COLOR_F {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 0.8,
        };
        let brush = target.CreateSolidColorBrush(&color, None).unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        presentation
            .draw_with_switches(&target, &brush, visual, switch_value)
            .unwrap();
        target.EndDraw(None, None).unwrap();
        let restored = brush.GetColor();
        assert_eq!(
            [restored.r, restored.g, restored.b, restored.a],
            [color.r, color.g, color.b, color.a]
        );
        let mut pixels = vec![0; 64 * 64 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 64 * 4, &mut pixels)
            .unwrap();
        // 后续不透明标记不受前一个场景的 opacity/clip 影响。
        target.BeginDraw();
        brush.SetColor(&D2D1_COLOR_F {
            r: 0.,
            g: 1.,
            b: 0.,
            a: 1.,
        });
        target.FillRectangle(
            &D2D_RECT_F {
                left: 60.,
                top: 60.,
                right: 64.,
                bottom: 64.,
            },
            &brush,
        );
        target.EndDraw(None, None).unwrap();
        let mut marked = vec![0; 64 * 64 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 64 * 4, &mut marked)
            .unwrap();
        assert_eq!(
            &marked[(62 * WIDTH + 62) * 4..(62 * WIDTH + 63) * 4],
            &[0, 255, 0, 255]
        );
        pixels
    }
}
