//! 显式授权的顶层窗口身份；持有原进程句柄，校验不依赖标题或屏幕位置。
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle};
use std::sync::Arc;
#[path = "live_input_backend.rs"]
pub mod input_backend;
#[path = "live_input_coordinates.rs"]
pub mod input_coordinates;
#[path = "live_input_message.rs"]
pub mod input_message;
#[path = "live_input_routing.rs"]
pub mod input_routing;
#[path = "live_input_security.rs"]
pub mod input_security;
#[path = "live_input_target.rs"]
pub mod input_target;
#[path = "live_recovery_target.rs"]
pub mod recovery;
#[path = "live_window_region.rs"]
pub mod region;
#[path = "live_window_snapshot.rs"]
pub mod snapshot;
#[path = "live_window_visibility.rs"]
pub mod visibility;
#[path = "live_window_watch.rs"]
mod watch;
pub(crate) use watch::Watcher;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    window: usize,
    process: u32,
    thread: u32,
}
#[derive(Clone)]
pub struct Target {
    identity: Identity,
    process: Arc<OwnedHandle>,
}
unsafe fn identity(hwnd: HWND) -> Result<Identity> {
    if !IsWindow(Some(hwnd)).as_bool() || GetAncestor(hwnd, GA_ROOT) != hwnd {
        return Err(Error::new(
            E_INVALIDARG,
            "Live target must be an existing top-level window",
        ));
    }
    let mut process = 0;
    let thread = GetWindowThreadProcessId(hwnd, Some(&mut process));
    if process == 0 || thread == 0 {
        return Err(Error::new(E_FAIL, "Live window identity unavailable"));
    }
    Ok(Identity {
        window: hwnd.0 as usize,
        process,
        thread,
    })
}
impl Target {
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
    pub(crate) fn process_handle(&self) -> BorrowedHandle<'_> {
        self.process.as_handle()
    }
    /// 调用方须先确认用户选中的 HWND；本方法不枚举、不猜测或重新选择窗口。
    pub fn bind(hwnd: HWND) -> Result<Self> {
        unsafe {
            let identity = identity(hwnd)?;
            let process = OpenProcess(PROCESS_SYNCHRONIZE, false, identity.process)?;
            let target = Self {
                identity,
                process: Arc::new(OwnedHandle::from_raw_handle(process.0)),
            };
            target.validate()?;
            Ok(target)
        }
    }
    pub fn hwnd(&self) -> HWND {
        HWND(self.identity.window as *mut _)
    }
    pub(crate) fn process_id(&self) -> u32 {
        self.identity.process
    }
    /// 初始输出定位使用物理可见外框；不把此矩形当成 WGC item/纹理的权威尺寸。
    pub fn frame_bounds(&self) -> Result<RECT> {
        self.validate()?;
        if unsafe { IsIconic(self.hwnd()).as_bool() } {
            return Err(Error::new(
                E_ABORT,
                "Live source minimized before placement",
            ));
        }
        let rect = input_coordinates::frame_bounds(self.hwnd())
            .map_err(|_| Error::new(E_FAIL, "Live source physical bounds unavailable"))?;
        self.validate()?;
        Ok(rect)
    }
    pub(crate) fn watch(&self, notify: Arc<dyn Fn() + Send + Sync>) -> Result<Watcher> {
        self.validate()?;
        let watcher = Watcher::install(self.identity, notify)?;
        self.validate()?;
        Ok(watcher)
    }
    /// 移动/标题变化不改变身份；旧进程退出，即使 PID 被复用也必须拒绝。
    /// 同一存活进程/线程内的 HWND 快速复用仍有启动竞态，WGC item Closed 负责已绑定源生命周期。
    pub fn validate(&self) -> Result<()> {
        unsafe {
            if WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) != WAIT_TIMEOUT
                || identity(self.hwnd())? != self.identity
            {
                return Err(Error::new(E_ABORT, "Live source window changed or exited"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                if IsWindow(Some(self.0)).as_bool() {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
    }
    #[test]
    fn live_placement_source_bounds_are_physical_and_follow_move_but_reject_close() {
        unsafe {
            use windows::Win32::UI::HiDpi::*;
            struct Dpi(DPI_AWARENESS_CONTEXT);
            impl Drop for Dpi {
                fn drop(&mut self) {
                    unsafe {
                        SetThreadDpiAwarenessContext(self.0);
                    }
                }
            }
            let old = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            assert!(!old.0.is_null());
            let _dpi = Dpi(old);
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("Otter bounds fixture"),
                WS_POPUP,
                -30000,
                -30000,
                128,
                96,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let window = Window(hwnd);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let target = Target::bind(hwnd).unwrap();
            for (x, y) in [(-30000, -30000), (-29900, -29800)] {
                SetWindowPos(hwnd, None, x, y, 128, 96, SWP_NOACTIVATE | SWP_NOZORDER).unwrap();
                let bounds = target.frame_bounds().unwrap();
                assert_eq!(
                    (bounds.left, bounds.top, bounds.right, bounds.bottom),
                    (x, y, x + 128, y + 96)
                );
            }
            let _ = ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
            assert!(IsIconic(hwnd).as_bool());
            assert!(target.frame_bounds().is_err());
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            assert!(!IsIconic(hwnd).as_bool());
            assert!(target.frame_bounds().is_ok());
            drop(window);
            assert!(target.frame_bounds().is_err());
        }
    }
    #[test]
    fn hidden_window_binding_survives_move_but_rejects_close_and_changed_identity() {
        use windows::Win32::System::Com::*;
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        let _apartment = Apartment;
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("target"),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let _window = Window(hwnd);
            let target = Target::bind(hwnd).unwrap();
            let mut target = std::thread::spawn(move || {
                target.validate().unwrap();
                target
            })
            .join()
            .unwrap();
            assert!(matches!(
                crate::capture_wgc::live::Session::window(
                    &target,
                    std::sync::Arc::new(crate::live_frames::Source::new())
                ),
                Err(crate::capture_wgc::live::Error::NoConsumers)
            ));
            let mut binding_ran = false;
            let rejected =
                crate::live_job::LiveJob::start_window_bound(Target::bind(hwnd).unwrap(), |_| {
                    binding_ran = true;
                    Err::<(), _>(std::io::Error::other("injected binding failure"))
                });
            assert!(
                binding_ran,
                "must reach binding rather than fail COM admission"
            );
            assert_eq!(
                rejected.err().unwrap().to_string(),
                "injected binding failure"
            );
            SetWindowPos(hwnd, None, -100, 30, 40, 50, SWP_NOACTIVATE | SWP_NOZORDER).unwrap();
            assert!(target.validate().is_ok());
            target.identity.thread = target.identity.thread.wrapping_add(1);
            assert!(target.validate().is_err());
            target.identity = identity(hwnd).unwrap();
            DestroyWindow(hwnd).unwrap();
            assert!(target.validate().is_err());
            assert!(crate::live_job::LiveJob::start_window_bound(
                target,
                |_| -> std::io::Result<()> { panic!("closed target must not bind") }
            )
            .is_err());
            assert!(Target::bind(HWND::default()).is_err());
        }
    }
    #[test]
    fn child_window_is_not_silently_promoted_to_parent() {
        unsafe {
            let parent = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                32,
                32,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let child = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_CHILD,
                0,
                0,
                10,
                10,
                Some(parent),
                None,
                None,
                None,
            )
            .unwrap();
            let _window = Window(parent);
            assert!(Target::bind(child).is_err());
            DestroyWindow(parent).unwrap();
        }
    }
}
