//! 通用 Grid → 真实 HWND/Presentation/D2D → 同几何命中；隐藏窗口不注入全局输入。
use super::*;
use serde_json::json;
use windows::Win32::{
    Graphics::{
        Direct2D::{Common::*, *},
        Dxgi::Common::*,
        Imaging::*,
    },
    System::Com::*,
};
fn grid(columns: &str, hidden: bool) -> Scene {
    Scene::parse(&serde_json::to_vec(&json!({"id":"grid","type":"grid",
        "layout":{"width":180,"height":64,"padding":4,"gap":4,"gridTemplateColumns":columns,"gridTemplateRows":"24px 24px"},
        "style":{"background":"#0000ff"},"children":[
            {"id":"first","type":"view","props":{"visible":!hidden},"layout":{"gridColumn":"1","gridRow":"1"},"style":{"background":"#ff0000"},"children":[{"id":"ink","type":"text","props":{"text":"Grid"},"style":{"color":"#ffffff"}}]},
            {"id":"second","type":"view","layout":{"gridColumn":"2","gridRow":"1"},"style":{"background":"#00ff00"},"events":{"click":"choose"}},
            {"id":"tail","type":"view","layout":{"gridColumn":"1/-1","gridRow":"2"},"style":{"background":"#ffff00"}}
        ]})).unwrap()).unwrap()
}
fn geometry(window: &SurfaceWindow, id: &str) -> Option<([f32; 2], [f32; 2])> {
    let host = window.state.host.borrow();
    host.as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()
        .iter()
        .find(|n| n.id() == id)
        .map(|n| (n.origin(), n.size()))
}
fn pixels(window: &SurfaceWindow) -> Vec<u8> {
    let host = window.state.host.borrow();
    let frame = host.as_ref().unwrap().presentation();
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
        let mut output = vec![0; 200 * 80 * 4];
        bitmap
            .CopyPixels(std::ptr::null(), 800, &mut output)
            .unwrap();
        output
    }
}
fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 200 + x) * 4..(y * 200 + x + 1) * 4]
}
#[test]
fn grid_real_hwnd_pixels_hits_replacement_rollback_hidden_resize_and_close() {
    let _com = super::super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(grid("80px minmax(0,1fr)", false), 240, 180).unwrap();
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    assert_eq!(geometry(&window, "first"), Some(([4., 4.], [80., 24.])));
    assert_eq!(geometry(&window, "second"), Some(([88., 4.], [88., 24.])));
    assert_eq!(geometry(&window, "tail"), Some(([4., 32.], [172., 24.])));
    let before = pixels(&window);
    assert_eq!(pixel(&before, 1, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&before, 80, 20), [0, 0, 255, 255]);
    assert_eq!(pixel(&before, 90, 6), [0, 255, 0, 255]);
    assert_eq!(pixel(&before, 10, 40), [0, 255, 255, 255]);
    assert!((4..28).any(|y| (4..60).any(|x| pixel(&before, x, y)[..3].iter().all(|v| *v > 220))));
    assert_eq!(window.hit_test([90, 6]).unwrap().as_deref(), Some("second"));
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    let events = window
        .click_events([90, 6], &binding, 7, Default::default())
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].revision(), 7);
    assert!(window
        .replace(grid("repeat(auto-fill,1fr)", false))
        .is_err());
    assert_eq!(pixels(&window), before);
    assert_eq!(window.hit_test([90, 6]).unwrap().as_deref(), Some("second"));
    window.replace(grid("60px minmax(0,1fr)", true)).unwrap();
    assert!(geometry(&window, "first").is_none() && geometry(&window, "ink").is_none());
    assert_eq!(geometry(&window, "second"), Some(([68., 4.], [108., 24.])));
    assert_eq!(pixel(&pixels(&window), 6, 6), [255, 0, 0, 255]);
    assert_eq!(window.hit_test([6, 6]).unwrap().as_deref(), Some("grid"));
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            280,
            200,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    assert_eq!(geometry(&window, "second"), Some(([68., 4.], [108., 24.])));
    let hwnd = window.hwnd;
    window.close().unwrap();
    assert!(!window.is_alive() && !unsafe { IsWindow(Some(hwnd)) }.as_bool());
    assert!(window.state.host.borrow().is_none());
    println!("native-grid-hwnd: pixels=true directwrite=true same-hit=true failed-replace-kept=true hidden=true resized=true closed=true");
}
