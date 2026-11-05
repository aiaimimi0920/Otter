//! 真实 WIC/D2D 与生产 Presentation/NativeMeasure；授权下载和 HWND 安装另验。
use super::*;
#[path = "surface_image_padding_pixels_tests.rs"]
mod padding;
#[path = "surface_image_placeholder_tests.rs"]
mod placeholder;
use crate::surface_scene::Scene;
use windows::Win32::{Graphics::Imaging::*, System::Com::*};
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
fn apartment() -> Apartment {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    Apartment
}
fn source() -> Rc<Bitmap> {
    Bitmap::new(
        Image::from_surface(
            &[255, 0, 0, 255, 0, 255, 0, 255],
            "application/x-neuro-rgba8",
            Some((2, 1)),
            || false,
        )
        .unwrap(),
    )
    .unwrap()
}
fn context() -> Context {
    Context {
        axis: 64.,
        viewport: [64., 64.],
        container: [64., 64.],
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}
fn target(factory: &ID2D1Factory) -> (IWICBitmap, ID2D1RenderTarget) {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(64, 64, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)
            .unwrap();
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
        (bitmap, target)
    }
}
fn pixels(bitmap: &IWICBitmap) -> Vec<u8> {
    let mut pixels = vec![0; 64 * 64 * 4];
    unsafe {
        bitmap
            .CopyPixels(std::ptr::null(), 256, &mut pixels)
            .unwrap();
    }
    pixels
}
fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4]
}
fn render(json: &[u8]) -> Vec<u8> {
    render_source(json, source())
}
fn render_source(json: &[u8], source: Rc<Bitmap>) -> Vec<u8> {
    let _com = apartment();
    let scene = Scene::parse(json).unwrap();
    let style = crate::surface_text::Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace: crate::surface_text::Whitespace::Normal,
        align: crate::surface_text::Align::Start,
        line_height: None,
    };
    let frame = crate::surface_presentation::Presentation::with_images(
        scene,
        style,
        context(),
        [64., 64.],
        [("image".to_owned(), source.clone())].into(),
    )
    .unwrap();
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let (bitmap, target) = target(&factory);
        let color = D2D1_COLOR_F {
            r: 0.25,
            g: 0.5,
            b: 0.75,
            a: 0.8,
        };
        let brush = target.CreateSolidColorBrush(&color, None).unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        frame.draw(&target, &brush).unwrap();
        target.EndDraw(None, None).unwrap();
        let restored = brush.GetColor();
        assert_eq!(
            [restored.r, restored.g, restored.b, restored.a],
            [color.r, color.g, color.b, color.a]
        );
        let result = pixels(&bitmap);
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
        assert_eq!(pixel(&pixels(&bitmap), 62, 62), [0, 255, 0, 255]);
        frame.release_device_resources();
        assert!(source.cache.borrow().is_none());
        source.bitmap(&target).unwrap();
        drop(frame);
        assert!(source.cache.borrow().is_none());
        result
    }
}
#[test]
fn surface_image_paint_contains_and_centers_without_stretch() {
    let p = render(br#"{"id":"image","type":"image","layout":{"width":64,"height":64}}"#);
    assert_eq!(pixel(&p, 8, 8), [0, 0, 0, 0]);
    assert_eq!(pixel(&p, 8, 32), [0, 0, 255, 255]);
    assert_eq!(pixel(&p, 56, 32), [0, 255, 0, 255]);
    assert_eq!(pixel(&p, 8, 56), [0, 0, 0, 0]);
}
#[test]
fn surface_image_fractional_logical_size_controls_real_contain_paint() {
    let tiny = Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    };
    for size in [
        [f32::from_bits(1); 2],
        [f32::from_bits(1), 1.],
        [1., f32::MIN_POSITIVE],
    ] {
        assert!(Bitmap::with_logical_size(tiny.clone(), size).is_err());
    }
    let image = Image::from_surface(
        &[255, 0, 0, 255].repeat(6),
        "application/x-neuro-rgba8",
        Some((3, 2)),
        || false,
    )
    .unwrap();
    for size in [
        [f32::NAN, 1.],
        [f32::INFINITY, 1.],
        [0., 1.],
        [3.1, 2.],
        [1., 1.],
    ] {
        assert!(Bitmap::with_logical_size(image.clone(), size).is_err());
    }
    let source = Bitmap::with_logical_size(image, [2.5, 1.5]).unwrap();
    assert_eq!(source.natural_size(), [2.5, 1.5]);
    let p = render_source(
        br#"{"id":"image","type":"image","layout":{"width":25,"height":25}}"#,
        source,
    );
    assert_eq!(pixel(&p, 10, 4), [0, 0, 0, 0]);
    assert_eq!(pixel(&p, 10, 6), [0, 0, 255, 255]);
    assert_eq!(pixel(&p, 10, 19), [0, 0, 255, 255]);
    assert_eq!(pixel(&p, 10, 21), [0, 0, 0, 0]);
    // 彩色列区分逻辑 sourceRect 与把完整 3 像素拉伸为 2.5 单位的错误路径。
    let row = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
    let image = Image::from_surface(
        &row.repeat(2),
        "application/x-neuro-rgba8",
        Some((3, 2)),
        || false,
    )
    .unwrap();
    let source = Bitmap::with_logical_size(image, [2.5, 1.5]).unwrap();
    let p = render_source(
        br#"{"id":"image","type":"image","layout":{"width":25,"height":25}}"#,
        source,
    );
    let center = pixel(&p, 14, 10);
    assert_eq!(
        center[0], 0,
        "sourceRect must not stretch the full raster: {center:?}"
    );
    assert!(
        center[1] > 220 && center[2] < 35 && center[3] == 255,
        "{center:?}"
    );
}
#[test]
fn surface_image_paint_obeys_opacity_sibling_order_and_rounded_mask() {
    let p=render(br#"{"id":"root","type":"view","layout":{"width":64,"height":64},"style":{"opacity":0.5},"children":[{"id":"image","type":"image","layout":{"width":64,"height":64}}]}"#);
    assert!((127..=128).contains(&pixel(&p, 8, 32)[3]));
    let p=render(br#"{"id":"root","type":"stack","layout":{"width":64,"height":64},"children":[{"id":"image","type":"image","layout":{"width":64,"height":64}},{"id":"cover","type":"view","layout":{"width":64,"height":64},"style":{"background":"blue"}}]}"#);
    assert!(p.chunks_exact(4).all(|pixel| pixel == [255, 0, 0, 255]));
    let p=render(br#"{"id":"image","type":"image","layout":{"width":64,"height":32},"style":{"borderRadius":16}}"#);
    assert_eq!(pixel(&p, 0, 0), [0, 0, 0, 0]);
    assert_eq!(pixel(&p, 8, 16), [0, 0, 255, 255]);
}
#[test]
fn surface_image_paint_padding_uses_content_edge_radius() {
    let p = render(br#"{"id":"image","type":"image","layout":{"width":64,"height":40,"padding":4},"style":{"borderRadius":16}}"#);
    // 内容区 56x32，contain 为 56x28；content radius=12，而不是 padding radius=16。
    assert_eq!(pixel(&p, 4, 6), [0, 0, 0, 0]);
    assert_eq!(pixel(&p, 8, 20), [0, 0, 255, 255]);
}
#[test]
fn surface_image_paint_cache_keys_exact_target_and_releases_on_clear() {
    let _com = apartment();
    let source = source();
    unsafe {
        let factory: ID2D1Factory =
            D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
        let (_a, a) = target(&factory);
        let (_b, b) = target(&factory);
        let first = source.bitmap(&a).unwrap();
        assert_eq!(first.as_raw(), source.bitmap(&a).unwrap().as_raw());
        let second = source.bitmap(&b).unwrap();
        assert_ne!(first.as_raw(), second.as_raw());
        assert_eq!(
            source.cache.borrow().as_ref().unwrap().target.as_raw(),
            b.as_raw()
        );
        source.clear();
        assert!(source.cache.borrow().is_none());
        let third = source.bitmap(&b).unwrap();
        assert_ne!(second.as_raw(), third.as_raw());
    }
    let mut invalid = source.image.clone();
    invalid.width = 3;
    assert_eq!(Bitmap::new(invalid).err().unwrap().code(), E_INVALIDARG);
}
