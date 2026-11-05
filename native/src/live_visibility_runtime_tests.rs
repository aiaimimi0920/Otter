//! 显式桌面门禁：仅捕获自建纯色源，验证真实 WGC 与登记/隐藏/召回生命周期。
use super::*;
use crate::{
    live_frames::Consumer, live_job::LiveJob, live_reclaim::IntentTracker, live_texture::Texture,
};
use std::time::{Duration, Instant};
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
}
impl Fixture {
    unsafe fn new() -> Self {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.VisibilityRuntime"),
                lpfnWndProc: Some(procedure),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            }),
            0
        );
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("Otter.Native.VisibilityRuntime"),
            w!("Otter visibility fixture"),
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
        let fixture = Self { hwnd, instance };
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        fixture.redraw(false);
        fixture
    }
    unsafe fn redraw(&self, black: bool) {
        SetClassLongPtrW(
            self.hwnd,
            GCLP_HBRBACKGROUND,
            GetStockObject(if black { BLACK_BRUSH } else { WHITE_BRUSH }).0 as isize,
        );
        assert!(InvalidateRect(Some(self.hwnd), None, true).as_bool());
        let _ = UpdateWindow(self.hwnd);
        DwmFlush().unwrap();
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, message, wp, lp)
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(w!("Otter.Native.VisibilityRuntime"), Some(self.instance));
        }
    }
}
struct Environment(DPI_AWARENESS_CONTEXT);
impl Environment {
    unsafe fn new() -> Self {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        Self(old)
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
            CoUninitialize();
        }
    }
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
    let data = std::slice::from_raw_parts((mapped.pData as *const u8).add(offset), 4);
    let result = [data[0], data[1], data[2], data[3]];
    context.Unmap(&staging, 0);
    result
}
unsafe fn frame(
    consumer: &mut Consumer<Texture>,
    job: &mut LiveJob<Texture>,
    owner: &mut Owner,
    protection: &Protection,
    black: bool,
    after: u64,
) -> u64 {
    let end = Instant::now() + Duration::from_secs(10);
    let mut observed = 0;
    loop {
        pump();
        owner.poll().unwrap();
        protection.poll().unwrap();
        if let Ok(Some(delivery)) = consumer.try_next() {
            let color = pixel(&delivery.frame.resource);
            if observed < 8 {
                eprintln!(
                    "visibility-frame: after={after} black={black} sequence={} size={}x{} pixel={color:?} iconic={} armed={} recalled={} bounds={:?}",
                    delivery.frame.sequence,
                    delivery.frame.resource.width,
                    delivery.frame.resource.height,
                    IsIconic(protection.source.target.hwnd()).as_bool(),
                    protection.source.armed.get(),
                    protection.source.recalled.get(),
                    protection.source.target.frame_bounds(),
                );
                observed += 1;
            }
            if delivery.frame.sequence > after
                && protection.source.armed.get()
                && delivery.frame.resource.width == 128
                && delivery.frame.resource.height == 96
                && color[3] >= 245
                && color[..3]
                    .iter()
                    .all(|value| if black { *value <= 10 } else { *value >= 245 })
            {
                return delivery.frame.sequence;
            }
        }
        assert!(
            Instant::now() < end,
            "WGC visibility frame timed out; armed={} ready={} iconic={} recalled={} bounds={:?} journal={:?} worker={:?}",
            protection.source.armed.get(),
            protection.source.ticket.ready(),
            IsIconic(protection.source.target.hwnd()).as_bool(),
            protection.source.recalled.get(),
            protection.source.target.frame_bounds(),
            owner.journal.as_ref().and_then(|journal| journal.fault()),
            job.take_ready()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "explicit owned-window WGC hide/reclaim gate; briefly shows a synthetic window"]
fn actual_wgc_hidden_source_updates_and_reclaim_keeps_capture_alive() {
    unsafe {
        let _environment = Environment::new();
        for minimized in [false, true] {
            eprintln!("visibility-case: minimized={minimized}");
            let fixture = Fixture::new();
            let target = Target::bind(fixture.hwnd).unwrap();
            let original_style = GetWindowLongPtrW(fixture.hwnd, GWL_EXSTYLE);
            if minimized {
                let _ = ShowWindow(fixture.hwnd, SW_MINIMIZE);
                DwmFlush().unwrap();
            }
            assert_eq!(IsIconic(fixture.hwnd).as_bool(), minimized);
            let mut owner = Owner::fixture();
            let protection = owner.acquire(target.clone()).unwrap();
            owner.observe_reclaim(2, None, 1, |_| minimized).unwrap();
            let (mut job, mut consumer) = LiveJob::start_window_bound(target.clone(), Ok).unwrap();
            let white = frame(&mut consumer, &mut job, &mut owner, &protection, false, 0);
            assert_eq!(Snapshot::read(&target).unwrap().layered.unwrap().alpha, 1);
            assert!(!IsIconic(fixture.hwnd).as_bool());
            fixture.redraw(true);
            let black = frame(
                &mut consumer,
                &mut job,
                &mut owner,
                &protection,
                true,
                white,
            );
            let mut intent = IntentTracker::default();
            intent.record(10, 2);
            owner
                .observe_reclaim(fixture.hwnd.0 as usize, intent.snapshot(11), 11, |_| false)
                .unwrap();
            assert!(protection.source.recalled.get());
            assert_eq!(GetWindowLongPtrW(fixture.hwnd, GWL_EXSTYLE), original_style);
            assert!(IsWindowVisible(fixture.hwnd).as_bool() && !IsIconic(fixture.hwnd).as_bool());
            fixture.redraw(false);
            let restored = frame(
                &mut consumer,
                &mut job,
                &mut owner,
                &protection,
                false,
                black,
            );
            assert!(restored > black && job.take_ready().is_none());
            drop(protection);
            drop(consumer);
            job.cancel();
            owner.close();
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut completed = false;
            while !completed || !owner.finished() {
                pump();
                owner.poll().unwrap();
                if let Some(result) = job.take_ready() {
                    assert_eq!(result, Err("Capture cancelled".into()));
                    completed = true;
                }
                assert!(
                    Instant::now() < deadline,
                    "Visibility runtime cleanup timed out"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            println!("native-live-visibility-runtime: minimized={minimized} hidden-frame=true update=true reclaim=true capture-continued=true reaped=true");
        }
    }
}
