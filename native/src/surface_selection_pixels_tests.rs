//! 真实 WIC/D2D 像素比较：高亮改变选区而不越过内容裁剪，绘制不泄漏画刷状态。
use super::*;
use windows::Win32::{
    Graphics::{Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM, Imaging::*},
    System::Com::*,
};
#[test]
fn text_selection_highlight_is_rendered_in_existing_frame_and_clipped() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"width":40,"height":40,"overflowX":"hidden"},"children":[{"id":"text","type":"text","props":{"text":"MMMMMMMM","selectable":true},"style":{"fontSize":20,"whiteSpace":"nowrap"},"layout":{"width":180,"height":40}}]}"#).unwrap();
    let owner = Presentation::new(scene, style(), context(), [200., 100.]).unwrap();
    assert_pixels(owner, [0.1, 8.], [1000., 8.], 40, 40, &[]);
}
#[test]
fn cross_node_highlights_all_three_items_in_real_pixels() {
    let owner = super::cross_node::cross_owner(false);
    let start = super::cross_node::point(&owner, "n0", false);
    let end = super::cross_node::point(&owner, "n2", true);
    assert_pixels(owner, start, end, 180, 30, &[0..60, 60..120, 120..180]);
}
fn assert_pixels(
    mut owner: Presentation,
    start: [f32; 2],
    end: [f32; 2],
    width: usize,
    height: usize,
    regions: &[std::ops::Range<usize>],
) {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
    }
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    let _com = Com;
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(
                200,
                100,
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
        let original = D2D1_COLOR_F {
            r: 0.3,
            g: 0.2,
            b: 0.1,
            a: 0.7,
        };
        let brush = target.CreateSolidColorBrush(&original, None).unwrap();
        let draw = |owner: &Presentation| {
            target.BeginDraw();
            target.Clear(Some(&D2D1_COLOR_F::default()));
            owner.draw(&target, &brush).unwrap();
            target.EndDraw(None, None).unwrap();
            let mut bytes = vec![0; 200 * 100 * 4];
            bitmap
                .CopyPixels(std::ptr::null(), 200 * 4, &mut bytes)
                .unwrap();
            bytes
        };
        let before = draw(&owner);
        assert!(owner.begin_text_selection(start).unwrap());
        owner.finish_text_selection(end).unwrap();
        let after = draw(&owner);
        let changed: Vec<_> = before
            .chunks_exact(4)
            .zip(after.chunks_exact(4))
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();
        assert!(changed.len() > 100);
        assert!(changed.iter().all(|i| i % 200 < width && i / 200 < height));
        for region in regions {
            assert!(changed.iter().any(|i| region.contains(&(i % 200))));
        }
        let color = brush.GetColor();
        assert_eq!(
            [color.r, color.g, color.b, color.a],
            [original.r, original.g, original.b, original.a]
        );
        owner.clear_text_selection();
        assert_eq!(draw(&owner), before);
    }
}
