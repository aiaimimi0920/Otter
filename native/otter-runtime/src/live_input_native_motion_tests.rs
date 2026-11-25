//! 宿主发布到自有源线程真实 hook，覆盖同步 motion 的租约与 modal 槽隔离。
use super::*;
use std::{cell::Cell, sync::mpsc, thread::JoinHandle};
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

thread_local! {
    static POPUP: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
const HIDE: u32 = WM_APP + 0x70;
const QUERY: u32 = WM_APP + 0x71;
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if matches!(message, WM_MOUSEMOVE | WM_MOUSEHOVER) {
        COUNT.with(|count| count.set(count.get() + 1));
        let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
        return LRESULT(0);
    }
    if message == HIDE {
        let _ = ShowWindow(POPUP.with(Cell::get), SW_HIDE);
        return LRESULT(0);
    }
    if message == QUERY {
        return LRESULT(COUNT.with(Cell::get) as isize);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
struct Window(HWND);
impl Window {
    fn new(class: windows::core::PCWSTR) -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                class,
                w!(""),
                WS_POPUP,
                -30000,
                -30000,
                64,
                64,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}
struct Source {
    thread: u32,
    worker: Option<JoinHandle<()>>,
    target: Destination,
    popup: HWND,
}
impl Source {
    fn new() -> Self {
        let (send, receive) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || unsafe {
            static CLASS: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            CLASS.get_or_init(|| {
                assert_ne!(
                    RegisterClassW(&WNDCLASSW {
                        lpfnWndProc: Some(procedure),
                        hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
                        lpszClassName: w!("Otter.Test.NativeMotionSource"),
                        ..Default::default()
                    }),
                    0
                );
            });
            let root = Window::new(w!("Otter.Test.NativeMotionSource"));
            let popup = Window::new(w!("STATIC"));
            POPUP.with(|slot| slot.set(popup.0));
            let thread = GetCurrentThreadId();
            let call = SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(otter_live_hover_hook::native_dispatch::otter_live_native_call),
                None,
                thread,
            )
            .unwrap();
            let returned = SetWindowsHookExW(
                WH_CALLWNDPROCRET,
                Some(otter_live_hover_hook::native_popup::otter_live_native_return),
                None,
                thread,
            )
            .unwrap();
            send.send((thread, root.0 .0 as usize, popup.0 .0 as usize))
                .unwrap();
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                DispatchMessageW(&message);
            }
            UnhookWindowsHookEx(returned).unwrap();
            UnhookWindowsHookEx(call).unwrap();
        });
        let (thread, root, popup) = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        let root = Target::bind(HWND(root as *mut _)).unwrap();
        let target = Destination::bind(&root, root.hwnd()).unwrap();
        Self {
            thread,
            worker: Some(worker),
            target,
            popup: HWND(popup as *mut _),
        }
    }
    fn count(&self) -> isize {
        unsafe { SendMessageW(self.target.hwnd(), QUERY, None, None).0 }
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0)).unwrap();
        }
        self.worker.take().unwrap().join().unwrap();
    }
}

#[test]
fn published_motion_is_executed_once_and_lease_outlives_request_not_epoch() {
    let source = Source::new();
    let b = Window::new(w!("STATIC"));
    let owner = crate::live_hover_owner::Owner::new(b.0).unwrap();
    unsafe {
        let _ = ShowWindow(b.0, SW_SHOWNOACTIVATE);
    }
    owner.control(true).unwrap();
    let mut modal = Modal::default();
    modal.set_owner(owner.identity().capture_control().unwrap());
    for (index, packet) in [
        Message::motion([4, 6], 0).unwrap(),
        Message::hover([4, 6], 0).unwrap(),
    ]
    .into_iter()
    .enumerate()
    {
        motion_native(&source.target, packet, 250, &mut modal).unwrap();
        assert_eq!(source.count(), (index + 1) as isize);
        let lease = otter_live_hover_hook::native_popup::Lease::read(source.popup).unwrap();
        assert_eq!(lease.owner, b.0);
        assert_eq!(lease.root, source.target.hwnd());
        assert_eq!(
            otter_live_hover_hook::native_protocol::value(
                b.0,
                otter_live_hover_hook::native_protocol::COMMIT
            ),
            0
        );
        unsafe {
            SendMessageW(source.target.hwnd(), HIDE, None, None);
        }
        assert!(!lease.valid());
    }
    modal.token = 12345;
    modal.destination = Some(source.target.clone());
    CALLBACK.with(|slot| slot.set((12345, false)));
    assert_eq!(
        motion_native(
            &source.target,
            Message::motion([1, 1], 0).unwrap(),
            250,
            &mut modal
        ),
        Err(Error::ModalPending)
    );
    assert_eq!(source.count(), 2);
    modal.clear();
    motion_native(
        &source.target,
        Message::hover([4, 6], 0).unwrap(),
        250,
        &mut modal,
    )
    .unwrap();
    let lease = otter_live_hover_hook::native_popup::Lease::read(source.popup).unwrap();
    owner.control(false).unwrap();
    assert!(!lease.valid());
}
