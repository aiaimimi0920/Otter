//! Wiring Native preparation to existing Leave filtering; owned windows only, without moving the system mouse.
use super::*;
use crate::live_input::Backend as _;
use otter_live_hover_hook::scope::*;
use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, POINT},
        UI::WindowsAndMessaging::*,
    },
};

struct Window(HWND);
impl Window {
    fn new(x: i32, y: i32) -> Self {
        Self(unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                // WindowFromPoint skips passive STATIC text; B must be an actually hittable window.
                WS_POPUP | WINDOW_STYLE(windows::Win32::System::SystemServices::SS_NOTIFY.0),
                x,
                y,
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

#[test]
#[ignore = "interactive desktop; own window under cursor and matching hook DLL required"]
fn native_preparation_binds_real_hover_and_does_not_steal_another_mirrors_slot() {
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).unwrap();
        let source = Window::new(-28000, -28000);
        let mirror = Window::new(cursor.x - 16, cursor.y - 16);
        SetWindowPos(
            mirror.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .unwrap();
        assert_eq!(GetAncestor(WindowFromPoint(cursor), GA_ROOT), mirror.0);
        let owner = crate::live_hover_owner::Owner::new(mirror.0).unwrap();
        owner.control(true).unwrap();
        owner.active(true).unwrap();
        let target = Target::bind(source.0).unwrap();
        let mut backend = Backend::new_local(target.clone(), None)
            .unwrap()
            .with_hover(Some(owner.identity().capture_control().unwrap()));
        assert!(!suppress(source.0));

        // Use production Native hooks/prepare; do not fabricate SOURCE/OWNER to hide missing wiring.
        backend.prepare_native().unwrap();
        assert_eq!(GetPropW(mirror.0, SOURCE).0, source.0 .0);
        assert_eq!(GetPropW(source.0, OWNER).0, mirror.0 .0);
        assert_eq!(GetPropW(source.0, SOURCE_COOKIE), owner.identity().token());
        assert!(suppress(source.0));

        let other_mirror = Window::new(-27000, -27000);
        let other_owner = crate::live_hover_owner::Owner::new(other_mirror.0).unwrap();
        other_owner.control(true).unwrap();
        other_owner.active(true).unwrap();
        let mut stale = Backend::new_local(target, None)
            .unwrap()
            .with_hover(Some(other_owner.identity().capture_control().unwrap()));
        stale.prepare_native().unwrap();
        assert_eq!(GetPropW(source.0, OWNER).0, mirror.0 .0);
        assert_eq!(GetPropW(source.0, SOURCE_COOKIE), owner.identity().token());
        assert!(GetPropW(other_mirror.0, SOURCE).is_invalid());
        assert_eq!(stale.leave().failed, 0);
        assert!(suppress(source.0));

        owner.active(false).unwrap();
        assert!(!suppress(source.0));
        owner.active(true).unwrap();
        backend.prepare_native().unwrap();
        assert!(suppress(source.0));
        assert_eq!(backend.leave().failed, 0);
        assert!(GetPropW(source.0, OWNER).is_invalid());
        assert!(GetPropW(source.0, SOURCE_COOKIE).is_invalid());
        assert!(GetPropW(mirror.0, SOURCE).is_invalid());
        assert!(!suppress(source.0));
    }
}
