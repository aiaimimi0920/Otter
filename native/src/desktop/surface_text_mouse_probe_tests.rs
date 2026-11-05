//! 显式系统鼠标探针：命中保护、带标记输入、完整 down/up 批次；不发布剪贴板。
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::UI::{HiDpi::*, Shell::*};
const MARKER: usize = 0x4f545458;
#[path = "surface_text_double_click_probe_tests.rs"]
mod double_click;
#[derive(Default, Debug)]
struct Trace {
    messages: Vec<u32>,
    overflow: bool,
}
unsafe extern "system" fn record(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if matches!(
        message,
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_LBUTTONUP
    ) && GetMessageExtraInfo().0 as usize == MARKER
    {
        let trace = &mut *(data as *mut Trace);
        if trace.messages.len() < 32 {
            trace.messages.push(message);
        } else {
            trace.overflow = true;
        }
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
struct Observer {
    hwnd: HWND,
    trace: Box<Trace>,
}
impl Observer {
    unsafe fn new(hwnd: HWND) -> Self {
        let mut trace = Box::<Trace>::default();
        assert!(SetWindowSubclass(
            hwnd,
            Some(record),
            MARKER,
            (&mut *trace as *mut Trace) as usize
        )
        .as_bool());
        Self { hwnd, trace }
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(record), MARKER);
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
unsafe fn pump() {
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        let mut msg = MSG::default();
        for _ in 0..128 {
            if !PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
unsafe fn mouse(point: POINT, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    assert!(width > 1 && height > 1);
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: (((point.x - left) as i64 * 65536 + 32768) / width as i64) as i32,
                dy: (((point.y - top) as i64 * 65536 + 32768) / height as i64) as i32,
                dwFlags: flags | MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                dwExtraInfo: MARKER,
                ..Default::default()
            },
        },
    }
}
unsafe fn send_inputs(inputs: &[INPUT]) {
    let sent = SendInput(inputs, std::mem::size_of::<INPUT>() as i32);
    // 批次正常包含完整 up；部分插入时仅释放本探针已插入的 down，避免留下按住状态。
    if sent != inputs.len() as u32 && sent >= 2 {
        let release = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    dwExtraInfo: MARKER,
                    ..Default::default()
                },
            },
        };
        let released = release_with(|| SendInput(&[release], std::mem::size_of::<INPUT>() as i32));
        assert!(
            released,
            "system rejected emergency LEFTUP; verify physical mouse state"
        );
    }
    assert_eq!(sent, inputs.len() as u32, "system input batch incomplete");
}
fn release_with(mut inject: impl FnMut() -> u32) -> bool {
    for _ in 0..3 {
        if inject() == 1 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    false
}
#[test]
fn text_mouse_probe_emergency_release_is_bounded_and_checks_result() {
    for success_at in [1, 2, 3, 4] {
        let mut calls = 0;
        let released = release_with(|| {
            calls += 1;
            u32::from(calls == success_at)
        });
        assert_eq!(released, success_at <= 3);
        assert_eq!(calls, success_at.min(3));
    }
}
#[test]
#[ignore = "requires explicit owned visible desktop input probe"]
fn text_selection_system_mouse_probe() {
    assert_eq!(
        std::env::var("OTTER_NATIVE_TEXT_SELECTION_PROBE").as_deref(),
        Ok("1")
    );
    let _com = crate::desktop::Apartment::new().unwrap();
    unsafe {
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        // 有用户按键或本线程已有捕获时拒绝运行；前台与实际命中另行校验。
        for key in [
            VK_LBUTTON, VK_RBUTTON, VK_MBUTTON, VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN,
        ] {
            assert!(GetAsyncKeyState(key.0 as i32) >= 0, "desktop input is busy");
        }
        assert!(GetCapture().is_invalid());
        let window = window();
        let observer = Observer::new(window.hwnd);
        window.show().unwrap();
        SetWindowPos(
            window.hwnd,
            Some(HWND_TOPMOST),
            400,
            300,
            260,
            180,
            SWP_SHOWWINDOW,
        )
        .unwrap();
        let _ = SetForegroundWindow(window.hwnd);
        pump();
        assert_eq!(GetForegroundWindow(), window.hwnd);
        let mut start = POINT { x: 1, y: 8 };
        let mut end = POINT { x: 210, y: 8 };
        assert!(ClientToScreen(window.hwnd, &mut start).as_bool());
        assert!(ClientToScreen(window.hwnd, &mut end).as_bool());
        assert_eq!(WindowFromPoint(start), window.hwnd);
        assert_eq!(WindowFromPoint(end), window.hwnd);
        let mut original = POINT::default();
        GetCursorPos(&mut original).unwrap();
        send_inputs(&[mouse(start, MOUSE_EVENT_FLAGS(0))]);
        pump();
        let mut cursor = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        GetCursorInfo(&mut cursor).unwrap();
        assert_eq!(cursor.hCursor, LoadCursorW(None, IDC_IBEAM).unwrap());
        assert_eq!(GetForegroundWindow(), window.hwnd);
        assert_eq!(WindowFromPoint(start), window.hwnd);
        assert_eq!(WindowFromPoint(end), window.hwnd);
        // SendInput 的单批输入不可与其他输入穿插；保留 move 以便证明生产拖选路径。
        send_inputs(&[
            mouse(start, MOUSE_EVENT_FLAGS(0)),
            mouse(start, MOUSEEVENTF_LEFTDOWN),
            mouse(end, MOUSEEVENTF_MOVE_NOCOALESCE),
            mouse(end, MOUSEEVENTF_LEFTUP),
        ]);
        pump();
        let mut current = POINT::default();
        GetCursorPos(&mut current).unwrap();
        if current.x == end.x && current.y == end.y && GetForegroundWindow() == window.hwnd {
            // 只恢复指针位置；绝不恢复/发布剪贴板，也不重新激活其他窗口。
            SetCursorPos(original.x, original.y).unwrap();
        }
        println!(
            "system mouse trace={:?}; selected={:?}",
            observer.trace,
            selected(&window)
        );
        assert!(!observer.trace.overflow);
        let trace = &observer.trace.messages;
        let down = trace.iter().position(|m| *m == WM_LBUTTONDOWN).unwrap();
        let up = trace.iter().position(|m| *m == WM_LBUTTONUP).unwrap();
        assert!(down < up && trace[down + 1..up].contains(&WM_MOUSEMOVE));
        assert_eq!(selected(&window).as_deref(), Some("Native selection"));
        assert_ne!(GetCapture(), window.hwnd);
        assert!(!window.has_pending_click());
        assert!(!window.has_pending_edit());
        assert!(window.take_error().is_none());
    }
}
