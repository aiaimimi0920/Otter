//! 显式运行的交互桌面验证：仅捕获本测试创建的纯色 HWND，不枚举窗口或读取桌面。
#![cfg(windows)]
use otter_native::{
    desktop::composition::live::Surface,
    live_frames::{Consumer, Frame},
    live_job::LiveJob,
    live_texture::Texture,
    live_window_target::Target,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Direct2D::Common::D2D_SIZE_U, Direct3D11::*, Dwm::DwmFlush, Gdi::*},
        System::{Com::*, LibraryLoader::GetModuleHandleW},
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};

struct Apartment;
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, message, wp, lp)
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Fixture {
    source: HWND,
    output: HWND,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new() -> Self {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.LiveRuntimeFixture"),
                lpfnWndProc: Some(procedure),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            }),
            0
        );
        let mut fixture = Self {
            source: HWND::default(),
            output: HWND::default(),
            instance,
        };
        fixture.source = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("Otter.Native.LiveRuntimeFixture"),
            w!("Otter Live runtime fixture"),
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
        fixture.output = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
            w!("STATIC"),
            w!("Otter Live runtime output"),
            WS_POPUP,
            0,
            0,
            96,
            96,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();
        SetWindowDisplayAffinity(fixture.output, WDA_EXCLUDEFROMCAPTURE).unwrap();
        let _ = ShowWindow(fixture.source, SW_SHOWNOACTIVATE);
        fixture.redraw();
        fixture
    }
    unsafe fn redraw(&self) {
        assert!(InvalidateRect(Some(self.source), None, true).as_bool());
        let _ = UpdateWindow(self.source);
        DwmFlush().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            for hwnd in [self.output, self.source] {
                if IsWindow(Some(hwnd)).as_bool() {
                    let _ = DestroyWindow(hwnd);
                }
            }
            let _ = UnregisterClassW(w!("Otter.Native.LiveRuntimeFixture"), Some(self.instance));
        }
    }
}
unsafe fn pump() {
    let mut message = MSG::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        assert_ne!(message.message, WM_QUIT, "unexpected fixture quit");
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}
unsafe fn center(frame: &Texture) -> [u8; 4] {
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
    let pixel = std::slice::from_raw_parts((mapped.pData as *const u8).add(offset), 4);
    let output = [pixel[0], pixel[1], pixel[2], pixel[3]];
    context.Unmap(&staging, 0);
    output
}
unsafe fn frame(
    consumer: &mut Consumer<Texture>,
    job: &mut LiveJob<Texture>,
    accept: impl Fn(&Texture, [u8; 4]) -> bool,
) -> Arc<Frame<Texture>> {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        pump();
        match consumer.try_next() {
            Ok(Some(delivery))
                if accept(&delivery.frame.resource, center(&delivery.frame.resource)) =>
            {
                return delivery.frame
            }
            Ok(_) => {}
            Err(error) => panic!(
                "capture source stopped: {error:?}; worker={:?}",
                job.take_ready()
            ),
        }
        assert!(
            Instant::now() < deadline,
            "fixture frame timed out; worker={:?}",
            job.take_ready()
        );
        // 仅显式测试的有界等待；生产捕获与 UI 不使用此轮询。
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "shows and captures only a synthetic fixture window; requires an interactive Windows desktop"]
fn actual_wgc_window_updates_resizes_presents_and_closes() {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let _apartment = Apartment;
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!previous.0.is_null());
        let _dpi = Dpi(previous);
        let mut fixture = Fixture::new();
        let target = Target::bind(fixture.source).unwrap();
        let (mut job, mut consumer) = LiveJob::start_window_bound(target, Ok).unwrap();
        let white = frame(&mut consumer, &mut job, |texture, pixel| {
            texture.width == 128 && texture.height == 96 && pixel.iter().all(|value| *value >= 245)
        });
        let mut surface = Surface::new(
            fixture.output,
            D2D_SIZE_U {
                width: 96,
                height: 96,
            },
            white.clone(),
        )
        .unwrap();
        surface.draw().unwrap();
        surface.present().unwrap();
        SetClassLongPtrW(
            fixture.source,
            GCLP_HBRBACKGROUND,
            GetStockObject(BLACK_BRUSH).0 as isize,
        );
        fixture.redraw();
        let black = frame(&mut consumer, &mut job, |_, pixel| {
            pixel[..3].iter().all(|value| *value <= 10) && pixel[3] >= 245
        });
        assert!(black.sequence > white.sequence);
        assert!(
            center(&white.resource).iter().all(|value| *value >= 245),
            "published snapshot was overwritten"
        );
        surface.set_frame(black.clone()).unwrap();
        SetWindowPos(
            fixture.source,
            None,
            20,
            20,
            256,
            144,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        fixture.redraw();
        let resized = frame(&mut consumer, &mut job, |texture, pixel| {
            texture.width == 256
                && texture.height == 144
                && pixel[..3].iter().all(|value| *value <= 10)
                && pixel[3] >= 245
        });
        assert!(resized.sequence > black.sequence);
        surface.set_frame(resized.clone()).unwrap();
        surface.draw().unwrap();
        surface.present().unwrap();
        let weak = Arc::downgrade(&resized);
        drop((white, black, resized, surface));
        DestroyWindow(fixture.source).unwrap();
        fixture.source = HWND::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = loop {
            pump();
            if let Some(result) = job.take_ready() {
                break result;
            }
            assert!(
                Instant::now() < deadline,
                "worker did not stop after fixture close"
            );
            std::thread::sleep(Duration::from_millis(5));
        };
        let error = result.expect_err("closed source must end the capture session");
        assert!(
            error.contains("SourceClosed") || error.contains("TargetChanged"),
            "{error}"
        );
        assert!(job.completion_handle().is_none());
        assert!(weak.upgrade().is_none());
        println!("native-live-window-runtime: white=true update=true snapshot-stable=true resize=true present=true close=true");
    }
}
