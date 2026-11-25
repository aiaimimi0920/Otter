//! Display-instance fixture: owned windows only, establishing leases through the real source hook rather than fabricated lease properties.
use super::*;
use otter_live_hover_hook::{native_dispatch, native_popup, native_protocol as p};
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use windows::{
    core::w,
    Win32::System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
};

thread_local! { static POPUP: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) }; }
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_RBUTTONUP {
        let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
pub struct Window(pub HWND);
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
                80,
                60,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None).unwrap().0)),
                None,
            )
            .unwrap()
        })
    }
    pub fn set_topmost(&self, enabled: bool) {
        unsafe {
            SetWindowPos(
                self.0,
                Some(if enabled {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                }),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
            .unwrap();
            // Check more than API return values: the test needs actual layering preconditions.
            assert_eq!(
                GetWindowLongPtrW(self.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0,
                enabled
            );
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0).unwrap();
        }
    }
}
pub struct Fixture {
    pub root: Window,
    pub popup: Window,
    pub b: Window,
    pub owner: crate::live_hover_owner::Owner,
    hooks: Vec<HHOOK>,
    _dll: Option<crate::live_native_hooks::Hooks>,
    next: Cell<usize>,
}
impl Fixture {
    pub fn new() -> Self {
        Self::create(false)
    }
    /// Cross-process recovery must share the actual DLL's TLS/Mutation with source input; fail immediately if the package is missing.
    pub fn with_dll() -> Self {
        Self::create(true)
    }
    fn create(dll: bool) -> Self {
        static CLASS: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        CLASS.get_or_init(|| unsafe {
            assert_ne!(
                RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
                    lpszClassName: w!("Otter.Test.PopupLayerRecovery"),
                    ..Default::default()
                }),
                0
            );
        });
        let root = Window::new(w!("Otter.Test.PopupLayerRecovery"));
        let popup = Window::new(w!("STATIC"));
        let b = Window::new(w!("STATIC"));
        let owner = crate::live_hover_owner::Owner::new(b.0).unwrap();
        unsafe {
            let _ = ShowWindow(b.0, SW_SHOWNOACTIVATE);
        }
        root.set_topmost(true);
        owner.control(true).unwrap();
        let native = dll.then(|| {
            crate::live_native_hooks::Hooks::install(
                &Target::bind(root.0).unwrap(),
                crate::live_native_hooks::Kind::Native,
            )
            .unwrap()
        });
        let hooks = if dll {
            Vec::new()
        } else {
            unsafe {
                vec![
                    SetWindowsHookExW(
                        WH_CALLWNDPROC,
                        Some(native_dispatch::otter_live_native_call),
                        None,
                        GetCurrentThreadId(),
                    )
                    .unwrap(),
                    SetWindowsHookExW(
                        WH_CALLWNDPROCRET,
                        Some(native_popup::otter_live_native_return),
                        None,
                        GetCurrentThreadId(),
                    )
                    .unwrap(),
                ]
            }
        };
        Self {
            root,
            popup,
            b,
            owner,
            hooks,
            _dll: native,
            next: Cell::new(303),
        }
    }
    pub fn show(&self) -> Lease {
        self.hide();
        POPUP.with(|slot| slot.set(self.popup.0));
        let nonce = self.next.get();
        self.next.set(nonce + 1);
        unsafe { SetPropW(self.b.0, p::COMMIT, Some(HANDLE::default())).unwrap() };
        otter_live_hover_hook::native_input_scope::Snapshot::default()
            .publish(self.b.0)
            .unwrap();
        unsafe {
            for (key, number) in [
                (p::COMMIT, 0),
                (p::COOKIE, self.owner.identity().cookie),
                (p::REQUEST_EPOCH, p::value(self.b.0, p::EPOCH)),
                (p::ROOT, self.root.0 .0 as usize),
                (p::DESTINATION, self.root.0 .0 as usize),
                (p::MESSAGE, WM_RBUTTONUP as usize),
                (p::WP, 0),
                (p::LP, 0),
                (p::CLAIMED, 0),
                (p::FINISHED, 0),
                (p::COMMIT, nonce),
            ] {
                SetPropW(self.b.0, key, Some(HANDLE(number as *mut _))).unwrap();
            }
            SendMessageW(
                self.root.0,
                native_dispatch::message_id(),
                Some(WPARAM(self.b.0 .0 as usize)),
                Some(LPARAM(nonce as isize)),
            );
        }
        assert_eq!(p::value(self.b.0, p::FINISHED), nonce);
        Lease::read(self.popup.0).unwrap()
    }
    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.popup.0, SW_HIDE);
        }
    }
    pub fn topmost(&self) -> bool {
        unsafe { GetWindowLongPtrW(self.popup.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
    }
    pub fn layered_without_attributes(&self) {
        use windows::Win32::Graphics::Gdi::*;
        unsafe {
            let style = GetWindowLongPtrW(self.popup.0, GWL_EXSTYLE);
            // Adding WS_EX_LAYERED alone may yield system-default attributes and does not imply a per-pixel window.
            // Explicitly submit a DIB to establish the same preconditions as a real UpdateLayeredWindow menu.
            SetWindowLongPtrW(
                self.popup.0,
                GWL_EXSTYLE,
                style & !(WS_EX_LAYERED.0 as isize),
            );
            SetWindowLongPtrW(self.popup.0, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
            assert_ne!(
                GetWindowLongPtrW(self.popup.0, GWL_EXSTYLE) as u32 & WS_EX_LAYERED.0,
                0
            );
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
            let mut bits = std::ptr::null_mut();
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 80,
                    biHeight: -60,
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0);
            let result = bitmap.and_then(|bitmap| {
                std::ptr::write_bytes(bits.cast::<u8>(), 255, 80 * 60 * 4);
                let previous = SelectObject(dc, bitmap.into());
                let result = UpdateLayeredWindow(
                    self.popup.0,
                    None,
                    None,
                    Some(&SIZE { cx: 80, cy: 60 }),
                    Some(dc),
                    Some(&POINT::default()),
                    COLORREF(0),
                    Some(&BLENDFUNCTION {
                        BlendOp: AC_SRC_OVER as u8,
                        SourceConstantAlpha: 255,
                        AlphaFormat: AC_SRC_ALPHA as u8,
                        ..Default::default()
                    }),
                    ULW_ALPHA,
                );
                SelectObject(dc, previous);
                let _ = DeleteObject(bitmap.into());
                result
            });
            let _ = DeleteDC(dc);
            result.unwrap();
            let (mut color, mut alpha, mut flags) = (
                COLORREF::default(),
                0,
                LAYERED_WINDOW_ATTRIBUTES_FLAGS::default(),
            );
            assert!(GetLayeredWindowAttributes(
                self.popup.0,
                Some(&mut color),
                Some(&mut alpha),
                Some(&mut flags)
            )
            .is_err());
        }
    }
    pub fn record(&self, lease: Lease) -> Record {
        let target = Target::bind(self.popup.0).unwrap();
        Recovery::bind(target, &wire(lease))
            .unwrap()
            .record()
            .unwrap()
    }
    pub fn borrow(&self, lease: Lease) {
        let recovery = Recovery::bind(Target::bind(self.popup.0).unwrap(), &wire(lease)).unwrap();
        settle(|| recovery.acquire());
        assert!(self.topmost());
        assert_eq!(
            p::value(
                self.popup.0,
                otter_live_hover_hook::native_popup_layer::MANAGED
            ),
            lease.instance
        );
    }
    pub fn detach_input(&mut self) {
        for hook in self.hooks.drain(..).rev() {
            unsafe {
                UnhookWindowsHookEx(hook).unwrap();
            }
        }
        self._dll.take();
    }
}

/// Only the test thread's owned-window queue; PM_REMOVE is required for the production source hook to execute posted requests.
pub fn pump() {
    unsafe {
        for _ in 0..128 {
            let mut message = MSG::default();
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            assert_ne!(
                message.message, WM_QUIT,
                "fixture must not consume shutdown"
            );
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
pub fn settle(mut operation: impl FnMut() -> Result<()>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match operation() {
            Ok(()) => return,
            Err(error) => assert_eq!(
                error.code(),
                otter_live_hover_hook::native_layer_protocol::PENDING,
                "unexpected popup layer result: {error:?}"
            ),
        }
        assert!(
            Instant::now() < deadline,
            "popup source request did not finish"
        );
        pump();
        std::thread::sleep(Duration::from_millis(1));
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.hide();
        self.detach_input();
    }
}
