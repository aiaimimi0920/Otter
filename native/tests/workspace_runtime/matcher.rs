//! 真实 Win32 STATIC 类名与退出菜单匹配；离屏自有 HWND，不请求前台或注入输入。
use super::*;

struct Window(HWND);
impl Window {
    unsafe fn new(owner: Option<HWND>, visible: bool) -> Self {
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("STATIC"),
            w!("Otter 菜单"),
            WS_POPUP,
            -16000,
            -16000,
            32,
            32,
            owner,
            None,
            None,
            None,
        )
        .unwrap();
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Self(hwnd)
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

#[test]
fn real_static_menu_discovery_preserves_pid_owner_and_visibility_checks() {
    unsafe {
        let owner = Window::new(None, false);
        let other = Window::new(None, false);
        let menu = Window::new(Some(owner.0), true);
        let mut class = [0u16; 64];
        let size = GetClassNameW(menu.0, &mut class);
        assert!(size > 0);
        println!(
            "native-menu-discovery-class={}",
            String::from_utf16_lossy(&class[..size as usize])
        );
        let mut search = Dialog {
            pid: std::process::id(),
            hwnd: None,
            menu: true,
            owner: owner.0,
        };
        let _ = dialog(menu.0, LPARAM(&mut search as *mut Dialog as isize));
        assert_eq!(search.hwnd.take(), Some(menu.0));
        search.owner = other.0;
        let _ = dialog(menu.0, LPARAM(&mut search as *mut Dialog as isize));
        assert!(search.hwnd.is_none(), "wrong owner must not match");
        search.owner = owner.0;
        search.pid = search.pid.checked_add(1).unwrap();
        let _ = dialog(menu.0, LPARAM(&mut search as *mut Dialog as isize));
        assert!(search.hwnd.is_none(), "wrong PID must not match");
        search.pid = std::process::id();
        let _ = ShowWindow(menu.0, SW_HIDE);
        let _ = dialog(menu.0, LPARAM(&mut search as *mut Dialog as isize));
        assert!(search.hwnd.is_none(), "hidden menu must not match");
    }
}
