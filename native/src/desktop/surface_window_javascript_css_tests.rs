//! 同 child → CSS → 生产 HWND/Presentation → DirectWrite/D2D 像素和同几何命中。
use super::*;
use crate::surface_presentation::Presentation;
use windows::Win32::{
    Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::*,
        Imaging::*,
    },
    System::Com::*,
};
pub(super) fn geometry(window: &SurfaceWindow, id: &str) -> Option<([f32; 2], [f32; 2])> {
    let host = window.state.host.borrow();
    host.as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()
        .iter()
        .find(|node| node.id() == id)
        .map(|node| (node.origin(), node.size()))
}
fn render(frame: &Presentation) -> Vec<u8> {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let bitmap = wic
            .CreateBitmap(
                200,
                80,
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
            .CreateSolidColorBrush(&D2D1_COLOR_F::default(), None)
            .unwrap();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        frame.draw(&target, &brush).unwrap();
        target.EndDraw(None, None).unwrap();
        let mut pixels = vec![0; 200 * 80 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 800, &mut pixels)
            .unwrap();
        pixels
    }
}
pub(super) fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 200 + x) * 4..(y * 200 + x + 1) * 4]
}
pub(super) fn pixels(window: &SurfaceWindow) -> Vec<u8> {
    let host = window.state.host.borrow();
    render(host.as_ref().unwrap().presentation())
}
#[test]
fn javascript_css_real_hwnd_layout_pixels_hits_click_hidden_and_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let binding = fixture::binding();
    let (mut session, frame) = Session::mount(
        binding.clone(),
        include_str!("../../tests/fixtures/javascript-css.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let first = frame.scene.root().children()[0].id().to_owned();
    let second = frame.scene.root().children()[1].id().to_owned();
    let window = SurfaceWindow::new(frame.scene, 200, 80).unwrap();
    assert_eq!(geometry(&window, &first), Some(([6., 6.], [32., 40.])));
    assert_eq!(geometry(&window, &second), Some(([46., 6.], [40., 40.])));
    let before = pixels(&window);
    assert_eq!(pixel(&before, 1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&before, 8, 8), [0, 255, 0, 255]);
    assert_eq!(pixel(&before, 48, 8), [0, 255, 0, 255]);
    // 确有来自继承 color 的白色字形，不以 Scene 的字符串代替 DirectWrite/D2D。
    assert!((10..42).any(|y| (7..36).any(|x| pixel(&before, x, y)[..3].iter().all(|v| *v > 220))));
    window.set_pointer_enabled(true).unwrap();
    window.javascript_queue_click_for_test(&first);
    let candidate = window.take_click().unwrap();
    assert_eq!(candidate.node_id, first);
    assert!(window
        .click_events_for_candidate(candidate, &binding, 3)
        .unwrap()
        .is_empty());
    let frame = session.click(&first, &|| false).unwrap();
    assert_eq!(frame.events.len(), 1);
    window.replace(frame.scene).unwrap();
    assert_eq!(geometry(&window, &first), Some(([6., 6.], [48., 40.])));
    assert!(geometry(&window, &second).is_none());
    let after = pixels(&window);
    assert_eq!(pixel(&after, 8, 8), [255, 0, 0, 255]);
    assert_eq!(pixel(&after, 70, 8), [16, 16, 16, 255]);
    {
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        assert_eq!(
            presentation.hit_test([70., 20.]),
            Some(presentation.scene().root().id())
        );
    }
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    assert_eq!(geometry(&window, &first), Some(([6., 6.], [60., 40.])));
    let root_width = {
        let host = window.state.host.borrow();
        host.as_ref().unwrap().presentation().geometry().nodes()[0].size()[0]
    };
    assert_eq!(
        geometry(&window, &second),
        Some(([74., 6.], [root_width - 80., 40.]))
    );
    assert_eq!(pixel(&pixels(&window), 8, 8), [0, 255, 255, 255]);
    window.javascript_queue_click_for_test(&second);
    assert_eq!(window.take_click().unwrap().node_id, second);
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-css-hwnd: geometry=true border-box=true pixels=true inherited-ink=true hidden-hits=true same-child=true closed=true");
}

#[test]
fn javascript_font_family_real_hwnd_uses_selected_metrics_pixels_and_same_child_update() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let _com = SurfaceWindow::javascript_apartment_for_test();
    let (mut session, frame) = Session::mount(
        fixture::binding(),
        include_str!("../../tests/fixtures/javascript-fonts.js").into(),
        fixture::snapshot(3, 42),
        fixture::resources(),
        &|| false,
    )
    .unwrap();
    let ids: Vec<_> = frame
        .scene
        .root()
        .children()
        .iter()
        .map(|n| n.id().to_owned())
        .collect();
    let window = SurfaceWindow::new(frame.scene, 200, 80).unwrap();
    let check = |families: [&str; 2]| {
        let host = window.state.host.borrow();
        let presentation = host.as_ref().unwrap().presentation();
        let engine = crate::surface_text::Engine::new().unwrap();
        for (id, family) in ids.iter().zip(families) {
            let item = presentation
                .text()
                .items()
                .iter()
                .find(|i| i.node_id() == id)
                .unwrap();
            let expected = engine
                .layout(
                    "WWii 0123",
                    crate::surface_text::Style {
                        family,
                        ..crate::surface_viewport::style()
                    },
                    item.allocated()[0],
                    32768.,
                )
                .unwrap();
            assert_eq!(item.layout().size(), expected.size());
            assert_eq!(item.layout().baseline(), expected.baseline());
            assert!(geometry(&window, id).is_some());
        }
    };
    check(["Consolas", "Segoe UI"]);
    let before = pixels(&window);
    assert!(before
        .chunks_exact(4)
        .any(|p| p[..3].iter().all(|v| *v > 220)));
    let frame = session
        .update(fixture::snapshot(4, 45), fixture::resources(), &|| false)
        .unwrap();
    window.replace(frame.scene).unwrap();
    check(["Segoe UI", "Consolas"]);
    let after = pixels(&window);
    assert_ne!(before, after);
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    session.dispose(&|| false).unwrap();
    window.close().unwrap();
    assert!(!window.is_alive());
    println!("native-javascript-font-family-hwnd: installed-order=true dwrite-metrics=true pixels=true same-child=true closed=true");
}
