//! 隐藏宿主默认不显示；显式可见门验证 NOACTIVATE、焦点、几何、失败恢复和隐藏意图。
use super::*;
use std::cell::Cell;
use windows::{core::w, Win32::UI::Input::KeyboardAndMouse::*};

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                DestroyWindow(self.0).unwrap();
            }
        }
    }
}
unsafe fn window() -> Window {
    Window(
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("STATIC"),
            w!("Otter visibility test"),
            WS_POPUP,
            48,
            48,
            64,
            64,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}

#[test]
fn hidden_host_and_revoked_owner_are_never_shown() {
    unsafe {
        let window = window();
        let active = Cell::new(true);
        let owns = || active.get();
        let allow = || true;
        let mut reset = Reset::begin(window.0, true, &owns, &allow).unwrap();
        assert!(!IsWindowVisible(window.0).as_bool());
        reset.restore().unwrap();
        assert!(!IsWindowVisible(window.0).as_bool());
        active.set(false);
        assert!(Reset::begin(window.0, true, &owns, &allow).is_err());
        assert!(!IsWindowVisible(window.0).as_bool());
    }
}

#[test]
#[ignore = "brief self-owned visible windows; no foreground request or input"]
fn visible_reset_preserves_focus_and_restores_on_error_without_showing_hidden_intent() {
    unsafe {
        let foreground = GetForegroundWindow();
        let window = window();
        let child = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            w!("probe"),
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            40,
            20,
            Some(window.0),
            None,
            None,
            None,
        )
        .unwrap();
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
        let _ = SetActiveWindow(window.0);
        let _ = SetFocus(Some(child));
        assert_eq!(GetActiveWindow(), window.0);
        assert_eq!(GetFocus(), child);
        let mut geometry = RECT::default();
        GetWindowRect(window.0, &mut geometry).unwrap();
        let owns = || IsWindow(Some(window.0)).as_bool();
        let allow = Cell::new(true);
        let allowed = || allow.get();
        {
            let _ordinary = Reset::begin(window.0, false, &owns, &allowed).unwrap();
            assert!(IsWindowVisible(window.0).as_bool());
        }
        let failed = || -> Result<()> {
            let _reset = Reset::begin(window.0, true, &owns, &allowed)?;
            assert!(!IsWindowVisible(window.0).as_bool());
            assert_eq!(GetActiveWindow(), window.0);
            assert_eq!(GetFocus(), child);
            assert_eq!(GetForegroundWindow(), foreground);
            Err(Error::new(E_FAIL, "candidate failure fixture"))
        };
        assert!(failed().is_err());
        assert!(IsWindowVisible(window.0).as_bool());
        assert_eq!(GetActiveWindow(), window.0);
        assert_eq!(GetFocus(), child);
        assert_eq!(GetForegroundWindow(), foreground);
        let mut after = RECT::default();
        GetWindowRect(window.0, &mut after).unwrap();
        assert_eq!(geometry, after);
        {
            let mut reset = Reset::begin(window.0, true, &owns, &allowed).unwrap();
            allow.set(false); // 重入选择分组/托盘隐藏，不得被恢复步骤反向显示。
            reset.restore().unwrap();
        }
        assert!(!IsWindowVisible(window.0).as_bool());
        let _ = ShowWindow(window.0, SW_SHOWNOACTIVATE);
        allow.set(true);
        let mut reset = Reset::begin(window.0, true, &owns, &allowed).unwrap();
        DestroyWindow(window.0).unwrap();
        reset.restore().unwrap();
        assert!(!IsWindow(Some(window.0)).as_bool());
    }
}

#[test]
#[ignore = "brief self-owned minimized window; no foreground request or input"]
fn minimized_reset_preserves_placement_without_activating() {
    unsafe {
        let foreground = GetForegroundWindow();
        let window = window();
        let _ = ShowWindow(window.0, SW_SHOWMINNOACTIVE);
        assert!(IsIconic(window.0).as_bool());
        let owns = || IsWindow(Some(window.0)).as_bool();
        Reset::begin(window.0, true, &owns, &|| true)
            .unwrap()
            .restore()
            .unwrap();
        assert!(IsIconic(window.0).as_bool());
        assert_eq!(GetForegroundWindow(), foreground);
    }
}
