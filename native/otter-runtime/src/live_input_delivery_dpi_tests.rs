//! 接收端坐标 oracle：已经映射的 lParam 不得因输入 worker 的默认 DPI 被二次缩放。
use super::*;
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::{System::LibraryLoader::GetModuleHandleW, UI::HiDpi::*},
};
thread_local! { static RECEIVED: RefCell<Vec<(u32,[i32;2])>> = const {RefCell::new(Vec::new())}; }
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(
        msg,
        WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEWHEEL | WM_MOUSEHWHEEL
    ) {
        RECEIVED.with(|received| {
            received
                .borrow_mut()
                .push((msg, [lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32]))
        });
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    unsafe fn enter(context: DPI_AWARENESS_CONTEXT) -> Self {
        let old = SetThreadDpiAwarenessContext(context);
        assert!(!old.0.is_null());
        Self(old)
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            let _ = SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.DeliveryDpi"), Some(self.0));
        }
    }
}

#[test]
#[ignore = "interactive desktop; sends only to owned hidden windows across caller and receiver DPI contexts"]
fn actual_already_mapped_mouse_delivery_preserves_receiver_coordinates_and_caller_dpi() {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpfnWndProc: Some(procedure),
                lpszClassName: w!("Otter.Native.DeliveryDpi"),
                ..Default::default()
            }),
            0
        );
        let _class = Class(instance);
        let contexts = [
            DPI_AWARENESS_CONTEXT_UNAWARE,
            DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ];
        for receiver in contexts {
            let _dpi = Dpi::enter(receiver);
            RECEIVED.with(|received| received.borrow_mut().clear());
            let window = Window(
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("Otter.Native.DeliveryDpi"),
                    w!(""),
                    WS_POPUP,
                    -30000,
                    -30000,
                    200,
                    100,
                    None,
                    None,
                    Some(instance),
                    None,
                )
                .unwrap(),
            );
            let target = Target::bind(window.0).unwrap();
            let timeout_target = target.clone();
            let timeout_caller_unaware =
                !AreDpiAwarenessContextsEqual(receiver, DPI_AWARENESS_CONTEXT_UNAWARE).as_bool();
            let worker = std::thread::spawn(move || {
                let destination = Destination::bind(&target, target.hwnd()).unwrap();
                for caller in [
                    DPI_AWARENESS_CONTEXT_UNAWARE,
                    DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
                    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
                ] {
                    let _dpi = Dpi::enter(caller);
                    let before = GetThreadDpiAwarenessContext();
                    send(&target, Message::motion([60, 35], 0).unwrap()).unwrap();
                    send_to(
                        &destination,
                        Message::button([60, 35], 1, Button::Left, true, false).unwrap(),
                    )
                    .unwrap();
                    release_to(
                        &destination,
                        Message::button([60, 35], 0, Button::Left, false, false).unwrap(),
                    )
                    .unwrap();
                    send_to(
                        &destination,
                        Message::wheel([310, 210], 0, 120, false).unwrap(),
                    )
                    .unwrap();
                    send_to(
                        &destination,
                        Message::wheel([310, 210], 0, -120, true).unwrap(),
                    )
                    .unwrap();
                    assert!(
                        AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext())
                            .as_bool()
                    );
                }
            });
            let deadline = Instant::now() + Duration::from_secs(5);
            while !worker.is_finished() {
                let mut message = MSG::default();
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&message);
                }
                assert!(Instant::now() < deadline, "DPI delivery worker timeout");
                std::thread::sleep(Duration::from_millis(1));
            }
            worker.join().unwrap();
            let received = RECEIVED.with(|received| received.borrow().clone());
            let expected = [
                (WM_MOUSEMOVE, [60, 35]),
                (WM_LBUTTONDOWN, [60, 35]),
                (WM_LBUTTONUP, [60, 35]),
                (WM_MOUSEWHEEL, [310, 210]),
                (WM_MOUSEHWHEEL, [310, 210]),
            ]
            .repeat(3);
            assert_eq!(received, expected, "receiver={receiver:?}");
            // 暂停此接收线程的泵，实际跨线程超时；不把超时误解释为绝对未送达。
            std::thread::spawn(move || {
                let caller = if timeout_caller_unaware {
                    DPI_AWARENESS_CONTEXT_UNAWARE
                } else {
                    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
                };
                let _dpi = Dpi::enter(caller);
                let before = GetThreadDpiAwarenessContext();
                let start = Instant::now();
                assert_eq!(
                    deliver(&timeout_target, Message::motion([60, 35], 0).unwrap(), 20),
                    Err(Error::DeliveryFailed)
                );
                assert!(start.elapsed() < Duration::from_secs(2));
                assert!(
                    AreDpiAwarenessContextsEqual(before, GetThreadDpiAwarenessContext()).as_bool()
                );
            })
            .join()
            .unwrap();
            eprintln!("native-input-delivery-dpi: receiver={receiver:?} callers=3 exact=true restored=true timeout-restored=true global-input=false");
        }
    }
}
