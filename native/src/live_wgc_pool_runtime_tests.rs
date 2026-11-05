//! 自有静止源与故意陈旧的池尺寸；无后续重绘也必须交付完整像素，超时先取消并 join。
use super::*;
use crate::{live_frames::Source, live_texture::Texture, live_window_target::Target};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Direct3D11::*, Dwm::DwmFlush, Gdi::*},
        System::{Com::*, LibraryLoader::GetModuleHandleW},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

struct Fixture {
    hwnd: HWND,
    instance: HINSTANCE,
    dpi: DPI_AWARENESS_CONTEXT,
}
impl Fixture {
    unsafe fn new() -> Self {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let dpi = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!dpi.0.is_null());
        let instance = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.StaticPoolRuntime"),
                lpfnWndProc: Some(procedure),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            }),
            0
        );
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("Otter.Native.StaticPoolRuntime"),
            w!("Otter static pool fixture"),
            WS_POPUP,
            20,
            20,
            128,
            96,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let fixture = Self {
            hwnd,
            instance,
            dpi,
        };
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        assert!(InvalidateRect(Some(hwnd), None, true).as_bool());
        let _ = UpdateWindow(hwnd);
        DwmFlush().unwrap();
        fixture
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(w!("Otter.Native.StaticPoolRuntime"), Some(self.instance));
            SetThreadDpiAwarenessContext(self.dpi);
            CoUninitialize();
        }
    }
}
unsafe extern "system" fn procedure(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(h, m, wp, lp)
}
unsafe fn pump() {
    let mut message = MSG::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        assert_ne!(message.message, WM_QUIT);
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}
unsafe fn pixel(frame: &Texture) -> [u8; 4] {
    let device = frame.texture().GetDevice().unwrap();
    let context = device.GetImmediateContext().unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    frame.texture().GetDesc(&mut desc);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .unwrap();
    let staging = staging.unwrap();
    context.CopyResource(&staging, frame.texture());
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .unwrap();
    let offset = (desc.Height / 2 * mapped.RowPitch + desc.Width / 2 * 4) as usize;
    let bytes = std::slice::from_raw_parts((mapped.pData as *const u8).add(offset), 4);
    let result = [bytes[0], bytes[1], bytes[2], bytes[3]];
    context.Unmap(&staging, 0);
    result
}

#[test]
#[ignore = "explicit owned-window WGC static resize gate; briefly shows a synthetic window"]
fn stale_pool_size_delivers_full_static_frame_without_source_redraw() {
    unsafe {
        let fixture = Fixture::new();
        for stale in [[237, 39], [64, 48], [256, 192]] {
            let target = Target::bind(fixture.hwnd).unwrap();
            let frames = Arc::new(Source::new());
            let mut consumer = frames.subscribe().unwrap();
            let (sender, receiver) = mpsc::sync_channel(1);
            let worker = std::thread::spawn(move || {
                let mut session = Session::window(&target, frames).unwrap();
                sender.send(session.stop_handle()).unwrap();
                // 强制重现 Size 在恢复前读取、实际内容已恢复的边界，不再重绘源。
                session
                    .recreate(SizeInt32 {
                        Width: stale[0],
                        Height: stale[1],
                    })
                    .unwrap();
                loop {
                    match session.next_frame() {
                        Ok(_) => {}
                        Err(Error::Cancelled) => return,
                        Err(error) => panic!("static pool capture: {error:?}"),
                    }
                }
            });
            let end = Instant::now() + Duration::from_secs(10);
            let mut stop = None;
            let mut captured = None;
            while Instant::now() < end && !worker.is_finished() {
                pump();
                if stop.is_none() {
                    stop = receiver.try_recv().ok();
                }
                if let Ok(Some(delivery)) = consumer.try_next() {
                    let image = &delivery.frame.resource;
                    captured = Some(([image.width, image.height], pixel(image)));
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            // 先唤醒采集线程再 join；失败也不留下永远阻塞在无帧通知上的 worker。
            if let Some(stop) = stop.or_else(|| receiver.try_recv().ok()) {
                stop.cancel();
            }
            drop(consumer);
            worker.join().unwrap();
            assert_eq!(
                captured,
                Some(([128, 96], [255, 255, 255, 255])),
                "stale={stale:?}"
            );
        }
    }
}
