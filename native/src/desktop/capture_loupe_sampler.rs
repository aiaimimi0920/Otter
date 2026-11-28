//! Background live desktop 15x15 sampler for the picker loupe; keeps DWM/GDI readbacks off the UI thread.
use super::{Bounds, Grid};
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::PostMessageW};

pub(super) const READY: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 0x664;

struct Request {
    wanted: Option<POINT>,
    stop: bool,
}

pub(super) struct Shared {
    request: Mutex<Request>,
    wake: Condvar,
    latest: Mutex<Option<Grid>>,
    notified: AtomicBool,
}

pub(super) struct Sampler {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

struct ScreenDc(HDC);
impl Drop for ScreenDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = ReleaseDC(None, self.0);
            }
        }
    }
}

struct PatchSurface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *const u8,
}

impl PatchSurface {
    unsafe fn new() -> Option<Self> {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 15,
                biHeight: -15,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bitmap) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        else {
            let _ = DeleteDC(dc);
            return None;
        };
        if bitmap.is_invalid() || bits.is_null() {
            if !bitmap.is_invalid() {
                let _ = DeleteObject(bitmap.into());
            }
            let _ = DeleteDC(dc);
            return None;
        }
        let previous = SelectObject(dc, bitmap.into());
        if previous.is_invalid() {
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(dc);
            return None;
        }
        Some(Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast(),
        })
    }

    unsafe fn sample(&self, start: POINT, displays: &[Bounds]) -> Option<Grid> {
        let screen = ScreenDc(GetDC(None));
        if screen.0.is_invalid() {
            return None;
        }
        BitBlt(
            self.dc,
            0,
            0,
            15,
            15,
            Some(screen.0),
            start.x,
            start.y,
            SRCCOPY,
        )
        .ok()?;
        let _ = GdiFlush();
        let patch = std::slice::from_raw_parts(self.bits, 225 * 4);
        Some(std::array::from_fn(|i| {
            let x = start.x.saturating_add((i % 15) as i32);
            let y = start.y.saturating_add((i / 15) as i32);
            displays
                .iter()
                .any(|[l, t, r, b]| x >= *l && x < *r && y >= *t && y < *b)
                .then_some(())?;
            let base = i * 4;
            Some([patch[base + 2], patch[base + 1], patch[base]])
        }))
    }
}

impl Drop for PatchSurface {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

impl Sampler {
    pub(super) fn start(
        hud_hwnd: HWND,
        displays: Vec<Bounds>,
        shared: Arc<Shared>,
    ) -> Option<Self> {
        let raw_hwnd = hud_hwnd.0 as isize;
        let worker_shared = shared.clone();
        let worker = std::thread::Builder::new()
            .name("otter-loupe".into())
            .spawn(move || unsafe {
                let Some(surface) = PatchSurface::new() else {
                    return;
                };
                let mut last_tick = Instant::now() - Duration::from_millis(16);
                loop {
                    let start = {
                        let mut guard = worker_shared.request.lock().unwrap();
                        while guard.wanted.is_none() && !guard.stop {
                            guard = worker_shared.wake.wait(guard).unwrap();
                        }
                        if guard.stop {
                            return;
                        }
                        guard.wanted.take().unwrap()
                    };
                    let elapsed = last_tick.elapsed();
                    if elapsed < Duration::from_millis(8) {
                        std::thread::sleep(Duration::from_millis(8) - elapsed);
                        let mut guard = worker_shared.request.lock().unwrap();
                        if guard.stop {
                            return;
                        }
                        let start = guard.wanted.take().unwrap_or(start);
                        drop(guard);
                        last_tick = Instant::now();
                        if let Some(grid) = surface.sample(start, &displays) {
                            *worker_shared.latest.lock().unwrap() = Some(grid);
                            if !worker_shared.notified.swap(true, Ordering::AcqRel) {
                                let target = HWND(raw_hwnd as *mut _);
                                if PostMessageW(Some(target), READY, WPARAM(0), LPARAM(0)).is_err()
                                {
                                    worker_shared.notified.store(false, Ordering::Release);
                                }
                            }
                        }
                        continue;
                    }
                    last_tick = Instant::now();
                    if let Some(grid) = surface.sample(start, &displays) {
                        *worker_shared.latest.lock().unwrap() = Some(grid);
                        if !worker_shared.notified.swap(true, Ordering::AcqRel) {
                            let target = HWND(raw_hwnd as *mut _);
                            if PostMessageW(Some(target), READY, WPARAM(0), LPARAM(0)).is_err() {
                                worker_shared.notified.store(false, Ordering::Release);
                            }
                        }
                    }
                }
            })
            .ok()?;
        Some(Self {
            shared,
            worker: Some(worker),
        })
    }

    pub(super) fn request(&self, start: POINT) {
        let mut guard = self.shared.request.lock().unwrap();
        guard.wanted = Some(start);
        self.shared.wake.notify_one();
    }
}

impl Shared {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            request: Mutex::new(Request {
                wanted: None,
                stop: false,
            }),
            wake: Condvar::new(),
            latest: Mutex::new(None),
            notified: AtomicBool::new(false),
        })
    }

    pub(super) fn take_latest(&self) -> Option<Grid> {
        self.notified.store(false, Ordering::Release);
        self.latest.lock().unwrap().take()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        {
            let mut guard = self.shared.request.lock().unwrap();
            guard.stop = true;
            self.shared.wake.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
