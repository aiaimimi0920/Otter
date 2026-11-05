//! 完整窗口输入链门禁；只捕获自建源，消息仅发送至自己的 Live HWND。
use super::*;
use crate::{
    desktop::Apartment,
    live_visibility_owner::Owner,
    live_window_target::{snapshot::Snapshot, Target},
};
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::{Direct3D11::*, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
};
thread_local! {
    static BLACK: Cell<bool> = const { Cell::new(false) };
    static PATTERN: Cell<bool> = const { Cell::new(false) };
    static INPUT: RefCell<Vec<(usize,u32,usize,bool)>> = const { RefCell::new(Vec::new()) };
    static POINTS: RefCell<Vec<(usize,u32,[i32;2])>> = const { RefCell::new(Vec::new()) };
}
unsafe extern "system" fn source_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(msg, WM_LBUTTONDOWN | WM_LBUTTONUP | WM_KEYDOWN | WM_KEYUP) {
        let hidden = GetWindowLongPtrW(GetAncestor(hwnd, GA_ROOT), GWL_EXSTYLE)
            & WS_EX_TRANSPARENT.0 as isize
            != 0;
        INPUT.with(|input| {
            input
                .borrow_mut()
                .push((hwnd.0 as usize, msg, wp.0, hidden))
        });
        if matches!(msg, WM_LBUTTONDOWN | WM_LBUTTONUP) {
            POINTS.with(|points| {
                points.borrow_mut().push((
                    hwnd.0 as usize,
                    msg,
                    [lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32],
                ))
            });
        }
        if msg == WM_LBUTTONUP {
            BLACK.set(!BLACK.get());
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
        return LRESULT(0);
    }
    if msg == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut paint);
        let brush = GetStockObject(if BLACK.get() && !PATTERN.get() {
            BLACK_BRUSH
        } else {
            WHITE_BRUSH
        });
        let _ = FillRect(dc, &paint.rcPaint, HBRUSH(brush.0));
        if PATTERN.get() {
            let brush = GetStockObject(if BLACK.get() {
                WHITE_BRUSH
            } else {
                BLACK_BRUSH
            });
            let _ = FillRect(
                dc,
                &RECT {
                    left: 30,
                    top: 20,
                    right: 90,
                    bottom: 50,
                },
                HBRUSH(brush.0),
            );
        }
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
struct Fixture {
    root: HWND,
    child: HWND,
    instance: HINSTANCE,
}
impl Fixture {
    unsafe fn new() -> Self {
        Self::with_style(WS_POPUP)
    }
    unsafe fn with_style(style: WINDOW_STYLE) -> Self {
        BLACK.set(false);
        PATTERN.set(false);
        INPUT.with(|input| input.borrow_mut().clear());
        POINTS.with(|points| points.borrow_mut().clear());
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("Otter.Native.InputRuntime"),
                lpfnWndProc: Some(source_proc),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            }),
            0
        );
        let root = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("Otter.Native.InputRuntime"),
            w!("Otter input fixture"),
            style,
            10,
            10,
            if style == WS_POPUP { 200 } else { 300 },
            if style == WS_POPUP { 100 } else { 180 },
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("Otter.Native.InputRuntime"),
            w!(""),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            200,
            100,
            Some(root),
            None,
            Some(instance),
            None,
        )
        .unwrap();
        let _ = ShowWindow(root, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(root);
        let _ = UpdateWindow(child);
        Self {
            root,
            child,
            instance,
        }
    }
    fn received(&self, msg: u32, key: Option<usize>) -> bool {
        INPUT.with(|input| {
            input.borrow().iter().any(|(window, message, code, _)| {
                *window == self.child.0 as usize
                    && *message == msg
                    && key.is_none_or(|key| *code == key)
            })
        })
    }
}

#[path = "composition_live_crop_runtime_tests.rs"]
mod crop;
#[path = "composition_live_cycles_runtime_tests.rs"]
mod cycles;
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.root);
            let _ = UnregisterClassW(w!("Otter.Native.InputRuntime"), Some(self.instance));
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
    let mut msg = MSG::default();
    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
        assert_ne!(msg.message, WM_QUIT);
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
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
    let color = [bytes[0], bytes[1], bytes[2], bytes[3]];
    context.Unmap(&staging, 0);
    color
}
unsafe fn until(
    session: &mut Session,
    owner: &mut Owner,
    label: &str,
    condition: impl Fn(&Session) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        pump();
        owner.poll().unwrap();
        assert!(session.poll().is_none(), "Live ended during {label}");
        if condition(session) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Live timeout during {label}; input={:?}",
            session.take_input_error()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
#[ignore = "interactive desktop; real WGC, own source/Live windows, real recovery child; no global input"]
fn actual_wgc_live_ui_input_feedback_focus_cancel_and_close_restore() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        assert!(!old.0.is_null());
        let _dpi = Dpi(old);
        let fixture = Fixture::new();
        let target = Target::bind(fixture.root).unwrap();
        let original = Snapshot::read(&target).unwrap();
        let mut owner = Owner::fixture();
        let protection = owner.acquire(target.clone()).unwrap();
        let mut session =
            Session::start_window(target.clone(), target.frame_bounds().unwrap()).unwrap();
        session.protect(protection);
        session.show().unwrap();
        until(&mut session, &mut owner, "white hidden source", |session| {
            let Some(frame) = session.window.as_ref().and_then(LiveWindow::frame_for_test) else {
                return false;
            };
            let color = pixel(&frame.resource);
            color[3] >= 245
                && color[..3].iter().all(|value| *value >= 245)
                && Snapshot::read(&target)
                    .unwrap()
                    .layered
                    .is_some_and(|layered| layered.alpha == 1)
        });
        let window = session.window.as_ref().unwrap().hwnd_for_test();
        let white = session
            .window
            .as_ref()
            .unwrap()
            .frame_for_test()
            .unwrap()
            .sequence;
        SendMessageW(window, WM_SYSCOMMAND, Some(WPARAM(0x1ff0)), None);
        until(&mut session, &mut owner, "enable", Session::input_enabled);
        let mut client = RECT::default();
        GetClientRect(window, &mut client).unwrap();
        let center =
            LPARAM(((client.right / 2) as u32 | (((client.bottom / 2) as u32) << 16)) as isize);
        SendMessageW(window, WM_LBUTTONDOWN, None, Some(center));
        SendMessageW(window, WM_LBUTTONUP, None, Some(center));
        until(&mut session, &mut owner, "input feedback", |session| {
            let frame = session.window.as_ref().unwrap().frame_for_test().unwrap();
            let color = pixel(&frame.resource);
            fixture.received(WM_LBUTTONUP, None)
                && frame.sequence > white
                && color[3] >= 245
                && color[..3].iter().all(|value| *value <= 10)
        });
        SendMessageW(window, WM_KEYDOWN, Some(WPARAM(65)), None);
        until(&mut session, &mut owner, "held key", |_| {
            fixture.received(WM_KEYDOWN, Some(65))
        });
        SendMessageW(window, WM_KILLFOCUS, None, None);
        until(&mut session, &mut owner, "focus release", |session| {
            !session.input_enabled()
                && session.input_completion_handle().is_none()
                && fixture.received(WM_KEYUP, Some(65))
        });
        SendMessageW(window, WM_SYSCOMMAND, Some(WPARAM(0x1ff0)), None);
        until(
            &mut session,
            &mut owner,
            "re-enable",
            Session::input_enabled,
        );
        SendMessageW(window, WM_LBUTTONDOWN, None, Some(center));
        SendMessageW(window, WM_KEYDOWN, Some(WPARAM(66)), None);
        until(&mut session, &mut owner, "close with held inputs", |_| {
            fixture.received(WM_KEYDOWN, Some(66))
        });
        session.stop();
        owner.close();
        let mut result = None;
        let deadline = Instant::now() + Duration::from_secs(12);
        while result.is_none() || !owner.finished() {
            pump();
            owner.poll().unwrap();
            if result.is_none() {
                result = session.poll();
            }
            assert!(Instant::now() < deadline, "Live close did not reap owners");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(result, Some(Ok(())));
        assert_eq!(session.input_cleanup().unwrap().released, 2);
        assert!(INPUT.with(
            |input| input.borrow().iter().any(|(hwnd, msg, key, hidden)| *hwnd
                == fixture.child.0 as usize
                && *msg == WM_KEYUP
                && *key == 66
                && *hidden)
        ));
        let restored = Snapshot::read(&target).unwrap();
        assert_eq!(restored.extended_style, original.extended_style);
        assert_eq!(restored.visible, original.visible);
        assert!(
            session.input_completion_handle().is_none() && session.completion_handle().is_none()
        );
        eprintln!("native-live-input-runtime: wgc-feedback=true ui-route=true focus-release=true release-before-restore=true reaped=true");
    }
}
