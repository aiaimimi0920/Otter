//! 联合测试独立绘制源；生产 WGC/保护 owner 负责采集与恢复，不注入系统输入。
use super::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::HiDpi::{
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

const CLASS: windows::core::PCWSTR = w!("Otter.Live.Joint.Wgc.Source");

struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut paint);
        let color = if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == 0 {
            COLORREF(0x001153d3)
        } else {
            COLORREF(0x00c11d47)
        };
        let brush = CreateSolidBrush(color);
        FillRect(dc, &paint.rcPaint, brush);
        let _ = DeleteObject(brush.into());
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}

pub(super) struct Capture {
    window: HWND,
    instance: HINSTANCE,
    target: crate::live_window_target::Target,
    original: crate::live_window_target::snapshot::Snapshot,
    changed: bool,
    _dpi: Dpi,
}

impl Capture {
    pub(super) unsafe fn start(app: &mut Application) -> Self {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let dpi = Dpi(old);
        let instance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .unwrap()
            .into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: CLASS,
                lpfnWndProc: Some(procedure),
                ..Default::default()
            }),
            0
        );
        let window = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Owned WGC Live source"),
            WS_POPUP,
            80,
            80,
            64,
            64,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(window);
        windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
        let target = crate::live_window_target::Target::bind(window).unwrap();
        let original = crate::live_window_target::snapshot::Snapshot::read(&target).unwrap();
        let capture = Self {
            window,
            instance,
            target,
            original,
            changed: false,
            _dpi: dpi,
        };
        app.live_visibility = crate::live_visibility_owner::Owner::fixture();
        let session = app.start_live_window(capture.target.clone(), None).unwrap();
        app.admit_live(session, false).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            crate::live_source_uia_test_window::Window::pump();
            app.live_visibility.poll().unwrap();
            app.live_sessions[0].poll();
            if app.live_sessions[0].snapshot_frame().unwrap().is_some() {
                break;
            }
            assert!(Instant::now() < deadline, "WGC source initial frame");
            std::thread::sleep(Duration::from_millis(5));
        }
        capture
    }

    pub(super) unsafe fn poll(&mut self, root: &std::path::Path, app: &mut Application) {
        app.live_visibility.poll().unwrap();
        if !self.changed && root.join("source-wgc-change").exists() {
            SetWindowLongPtrW(self.window, GWLP_USERDATA, 1);
            SetWindowPos(
                self.window,
                None,
                0,
                0,
                96,
                80,
                SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            assert!(InvalidateRect(Some(self.window), None, false).as_bool());
            let _ = UpdateWindow(self.window);
            windows::Win32::Graphics::Dwm::DwmFlush().unwrap();
            self.changed = true;
        }
    }

    pub(super) fn verify(&self) {
        let restored = crate::live_window_target::snapshot::Snapshot::read(&self.target).unwrap();
        assert_eq!(restored.extended_style, self.original.extended_style);
        assert_eq!(restored.visible, self.original.visible);
        assert_eq!(
            restored.placement.rcNormalPosition,
            self.original.placement.rcNormalPosition
        );
        assert!(self.changed);
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.window);
            let _ = UnregisterClassW(CLASS, Some(self.instance));
        }
    }
}
