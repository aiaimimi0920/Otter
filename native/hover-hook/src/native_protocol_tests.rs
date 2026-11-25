//! 标量协议使用自有窗口/源线程 hook；无全局键鼠输入，不访问用户应用。
use super::*;
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

#[path = "native_completion_tests.rs"]
mod completion;
#[path = "native_input_scope_tests.rs"]
mod input_scope;
#[path = "native_layer_transfer_tests.rs"]
mod layer_transfer;
#[path = "native_motion_tests.rs"]
mod motion;
#[path = "native_dispatch_nested_tests.rs"]
mod nested;

thread_local! {
    static BUTTON_HANDLER: std::cell::Cell<Option<fn()>> = const { std::cell::Cell::new(None) };
    static BUTTONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static POPUP: std::cell::Cell<HWND> = const { std::cell::Cell::new(HWND(std::ptr::null_mut())) };
    static POSITION_HANDLER: std::cell::Cell<Option<fn(HWND)>> = const { std::cell::Cell::new(None) };
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_WINDOWPOSCHANGING {
        if let Some(handler) = POSITION_HANDLER.with(std::cell::Cell::get) {
            handler(hwnd);
        }
    }
    if matches!(
        msg,
        WM_RBUTTONUP | WM_MOUSEMOVE | windows::Win32::UI::Controls::WM_MOUSEHOVER
    ) {
        BUTTONS.with(|count| count.set(count.get() + 1));
        if let Some(handler) = BUTTON_HANDLER.with(std::cell::Cell::get) {
            handler();
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

struct Window(HWND);
impl Window {
    fn new(parent: Option<HWND>) -> Self {
        static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        REGISTERED.get_or_init(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(wndproc),
                    hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
                    lpszClassName: w!("Otter.Test.NativeInputProtocol"),
                    ..Default::default()
                }),
                0
            );
        });
        Self(unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("Otter.Test.NativeInputProtocol"),
                w!(""),
                if parent.is_some() { WS_CHILD } else { WS_POPUP },
                0,
                0,
                1,
                1,
                parent,
                None,
                Some(HINSTANCE(GetModuleHandleW(None).unwrap().0)),
                None,
            )
            .unwrap()
        })
    }
    fn make_topmost(&self) {
        unsafe {
            SetWindowPos(
                self.0,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .unwrap();
            // Win32 成功返回不等于样式已设置；失败应停在夹具初始化。
            assert_ne!(
                GetWindowLongPtrW(self.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                0
            );
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            // 有些按钮回归会让源在自己的窗口过程中销毁原控件/窗口。
            if IsWindow(Some(self.0)).as_bool() {
                DestroyWindow(self.0).unwrap();
            }
        }
    }
}
fn put(hwnd: HWND, key: PCWSTR, val: usize) {
    unsafe {
        SetPropW(hwnd, key, Some(HANDLE(val as *mut _))).unwrap();
    }
}
struct Fixture {
    owner: Window,
    child: Window,
    root: Window,
}
impl Fixture {
    fn new() -> Self {
        let owner = Window::new(None);
        let root = Window::new(None);
        let child = Window::new(Some(root.0));
        unsafe {
            let _ = ShowWindow(owner.0, SW_SHOWNOACTIVATE);
        }
        put(owner.0, scope::COOKIE, 101);
        put(owner.0, EPOCH, 202);
        let fixture = Self { owner, child, root };
        fixture.publish(303);
        fixture
    }
    fn publish(&self, nonce: usize) {
        let owner = self.owner.0;
        put(owner, COMMIT, 0);
        for (key, val) in [
            (COOKIE, 101),
            (REQUEST_EPOCH, 202),
            (ROOT, self.root.0 .0 as usize),
            (DESTINATION, self.child.0 .0 as usize),
            (MESSAGE, WM_RBUTTONUP as usize),
            (WP, 0),
            (LP, 0xffff0001),
            (CLAIMED, 0),
            (FINISHED, 0),
        ] {
            put(owner, key, val);
        }
        crate::native_input_scope::Snapshot::default()
            .publish(owner)
            .unwrap();
        put(owner, COMMIT, nonce);
    }
    fn read(&self) -> Option<Request> {
        Request::read(self.owner.0, 303, self.child.0)
    }
}

#[test]
fn packet_allowlist_excludes_commands_pointers_and_reserved_bits() {
    for msg in [WM_LBUTTONDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP] {
        assert!(valid_packet(msg, 0x7f, u32::MAX as isize));
    }
    for msg in [
        WM_COMMAND,
        WM_COPYDATA,
        WM_CONTEXTMENU,
        WM_KEYDOWN,
        WM_RBUTTONDOWN,
    ] {
        assert!(!valid_packet(msg, 0, 0));
    }
    assert!(!valid_packet(WM_RBUTTONUP, 0x80, 0));
    assert!(!valid_packet(WM_RBUTTONUP, 0, -1));
    if usize::BITS > 32 {
        assert!(!valid_packet(WM_RBUTTONUP, 0, (u32::MAX as isize) + 1));
    }
}

#[test]
fn exact_transaction_is_claimed_once_and_a_new_nonce_is_independent() {
    let f = Fixture::new();
    let request = f.read().unwrap();
    assert!(request.claim().unwrap());
    assert!(!request.claim().unwrap());
    f.publish(304);
    assert!(!request.claim().unwrap());
    assert!(Request::read(f.owner.0, 304, f.child.0)
        .unwrap()
        .claim()
        .unwrap());
}

#[test]
fn revoked_epoch_cookie_and_partial_publication_never_execute() {
    let f = Fixture::new();
    let old = f.read().unwrap();
    for (key, bad, good) in [
        (EPOCH, 0, 202),
        (EPOCH, 203, 202),
        (scope::COOKIE, 102, 101),
        (COMMIT, 0, 303),
        (COOKIE, 0, 101),
    ] {
        put(f.owner.0, key, bad);
        assert!(f.read().is_none());
        assert!(!old.claim().unwrap());
        put(f.owner.0, key, good);
    }
    unsafe {
        let _ = ShowWindow(f.owner.0, SW_HIDE);
    }
    assert!(f.read().is_none());
}

#[test]
fn unrelated_window_or_source_thread_cannot_consume_a_request() {
    let f = Fixture::new();
    let unrelated = Window::new(None);
    assert!(Request::read(f.owner.0, 303, unrelated.0).is_none());
    put(f.owner.0, DESTINATION, unrelated.0 .0 as usize);
    assert!(Request::read(f.owner.0, 303, unrelated.0).is_none());
    f.publish(303);
    let (owner, child) = (f.owner.0 .0 as usize, f.child.0 .0 as usize);
    assert!(std::thread::spawn(move || Request::read(
        HWND(owner as *mut _),
        303,
        HWND(child as *mut _)
    )
    .is_none())
    .join()
    .unwrap());
}

#[test]
fn changed_packet_cannot_be_claimed_from_an_old_snapshot() {
    let f = Fixture::new();
    let old = f.read().unwrap();
    for (key, replacement, original) in [
        (MESSAGE, WM_LBUTTONUP as usize, WM_RBUTTONUP as usize),
        (WP, 1, 0),
        (LP, 42, 0xffff0001),
        (DESTINATION, f.root.0 .0 as usize, f.child.0 .0 as usize),
    ] {
        put(f.owner.0, key, replacement);
        assert!(!old.claim().unwrap());
        assert_eq!(value(f.owner.0, CLAIMED), 0);
        put(f.owner.0, key, original);
    }
    assert!(old.claim().unwrap());
}

#[test]
fn missing_owner_and_zero_nonce_are_never_valid_transactions() {
    let f = Fixture::new();
    assert!(Request::read(f.owner.0, 0, f.child.0).is_none());
    assert!(Request::read(HWND::default(), 303, f.child.0).is_none());
    put(f.owner.0, ROOT, 0);
    assert!(f.read().is_none());
    f.publish(303);
    put(f.owner.0, MESSAGE, WM_COPYDATA as usize);
    assert!(f.read().is_none());
    f.publish(303);
    put(f.owner.0, WP, 0x80);
    assert!(f.read().is_none());
}

#[test]
fn dispatcher_executes_once_rejects_nested_dispatch_and_closes_collection_at_pump() {
    use crate::native_dispatch as dispatch;
    let f = Fixture::new();
    BUTTONS.with(|count| count.set(0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| {
            let request = dispatch::collecting().unwrap();
            assert!(
                !dispatch::dispatch(request.owner, request.nonce, request.destination).unwrap()
            );
            dispatch::message_pump_entered();
            assert!(dispatch::collecting().is_none());
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert_eq!(BUTTONS.with(std::cell::Cell::get), 1);
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert!(dispatch::collecting().is_none());
    assert!(!dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(BUTTONS.with(std::cell::Cell::get), 1);
}

#[test]
fn revocation_during_application_callback_stops_collection_but_keeps_its_completion() {
    use crate::native_dispatch as dispatch;
    let f = Fixture::new();
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| {
            let request = dispatch::collecting().unwrap();
            put(request.owner, EPOCH, 0);
            assert!(dispatch::collecting().is_none());
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert!(
        f.read().is_none(),
        "completion must not authorize another input"
    );
    f.publish(304);
    put(f.owner.0, EPOCH, 202);
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 304);
}

struct Hooks(Vec<HHOOK>);
impl Hooks {
    fn new() -> Self {
        let mut hooks = Self(Vec::new());
        unsafe {
            for (kind, callback) in [
                (
                    WH_CALLWNDPROC,
                    crate::native_dispatch::otter_live_native_call
                        as unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT,
                ),
                (
                    WH_CALLWNDPROCRET,
                    crate::native_popup::otter_live_native_return,
                ),
            ] {
                hooks.0.push(
                    SetWindowsHookExW(kind, Some(callback), None, GetCurrentThreadId()).unwrap(),
                );
            }
        }
        hooks
    }
}
impl Drop for Hooks {
    fn drop(&mut self) {
        for hook in self.0.drain(..).rev() {
            unsafe {
                UnhookWindowsHookEx(hook).unwrap();
            }
        }
    }
}

#[test]
fn synchronous_popup_lease_survives_button_return_but_not_hide_reshow_or_epoch() {
    use crate::{native_dispatch as dispatch, native_popup::Lease};
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    POPUP.with(|slot| slot.set(popup.0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    let first = Lease::read(popup.0).unwrap();
    assert_eq!(first.owner, f.owner.0);
    assert_eq!(first.root, f.root.0);
    assert_eq!(first.request, 303);
    assert!(first.valid());
    unsafe {
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    assert!(!first.valid());
    unsafe {
        let _ = ShowWindow(popup.0, SW_SHOWNOACTIVATE);
    }
    assert!(Lease::read(popup.0).is_none());
    unsafe {
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    f.publish(304);
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    let second = Lease::read(popup.0).unwrap();
    assert_ne!(first.instance, second.instance);
    assert!(!first.valid());
    put(f.owner.0, EPOCH, 0);
    assert!(!second.valid());
}

#[test]
fn existing_visible_popup_and_show_after_message_pump_are_not_authorized() {
    use crate::{native_dispatch as dispatch, native_popup::Lease};
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    POPUP.with(|slot| slot.set(popup.0));
    unsafe {
        let _ = ShowWindow(popup.0, SW_SHOWNOACTIVATE);
    }
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            SetWindowPos(
                POPUP.with(std::cell::Cell::get),
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            )
            .unwrap();
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert!(Lease::read(popup.0).is_none());
    unsafe {
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            dispatch::message_pump_entered();
            let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
        }))
    });
    f.publish(304);
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert!(Lease::read(popup.0).is_none());
}

#[test]
fn registered_entry_with_duplicate_hooks_executes_one_button() {
    let f = Fixture::new();
    let _first = Hooks::new();
    let _second = Hooks::new();
    BUTTONS.with(|count| count.set(0));
    unsafe {
        SendMessageW(
            f.child.0,
            crate::native_dispatch::message_id(),
            Some(WPARAM(f.owner.0 .0 as usize)),
            Some(LPARAM(303)),
        );
    }
    assert_eq!(BUTTONS.with(std::cell::Cell::get), 1);
    assert_eq!(value(f.owner.0, CLAIMED), 303);
    assert_eq!(value(f.owner.0, FINISHED), 303);
}

#[test]
fn source_topmost_menu_restores_on_hide_and_after_epoch_revocation() {
    use crate::native_dispatch as dispatch;
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    f.root.make_topmost();
    POPUP.with(|slot| slot.set(popup.0));
    BUTTONS.with(|count| count.set(0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let popup = POPUP.with(std::cell::Cell::get);
            let _ = ShowWindow(popup, SW_SHOWNOACTIVATE);
            let promoted = GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
            let _ = ShowWindow(popup, SW_HIDE);
            let restored = GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 == 0;
            BUTTONS.with(|count| count.set(usize::from(promoted && restored)));
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(BUTTONS.with(std::cell::Cell::get), 1);
    f.publish(304);
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let request = dispatch::collecting().unwrap();
            let popup = POPUP.with(std::cell::Cell::get);
            let _ = ShowWindow(popup, SW_SHOWNOACTIVATE);
            let promoted = GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
            BUTTONS.with(|count| count.set(usize::from(promoted)));
            put(request.owner, EPOCH, 0);
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert_eq!(BUTTONS.with(std::cell::Cell::get), 1);
    unsafe {
        assert_eq!(
            GetWindowLongPtrW(popup.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
            0
        );
        assert_eq!(GetWindowLongPtrW(popup.0, GWLP_HWNDPARENT), 0);
        assert_ne!(
            GetWindowLongPtrW(f.root.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
            0
        );
    }
}
