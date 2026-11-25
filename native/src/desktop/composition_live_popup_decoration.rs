//! Exclude only confirmed menu-decoration windows; class names classify presentation, never replace source-window authorization.
use crate::live_window_target::{popup::Scope, Target};
use windows::Win32::UI::WindowsAndMessaging::*;

pub(super) fn is_decoration(
    root: &Target,
    owner: Option<crate::live_hover_owner::Identity>,
    scope: &Scope,
    parents: &[(&Scope, Option<crate::live_hover_owner::Identity>)],
) -> bool {
    let Ok(target) = scope.target() else {
        return false;
    };
    let mut class = [0u16; 64];
    let length = unsafe { GetClassNameW(target.hwnd(), &mut class) } as usize;
    // Real logs confirm this class is menu-owned and shadow-only; do not generalize to all layered/transparent windows.
    if !class[..length]
        .iter()
        .copied()
        .eq("PotShadowWnd".encode_utf16())
    {
        return false;
    }
    let Ok(menu) = (unsafe { GetWindow(target.hwnd(), GW_OWNER) }) else {
        return false;
    };
    // Remove attached decorations only when the menu itself can enter the same authorized set; ordinary sources are not menus.
    Scope::bind(root, owner, menu).is_ok_and(|menu| menu.target().is_ok())
        || parents.iter().any(|(parent, mirror)| {
            parent.target().is_ok_and(|target| target.hwnd() == menu)
                || parent.bind_descendant(*mirror, menu).is_ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        core::w,
        Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW},
    };

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, message, wp, lp)
    }

    struct Window(HWND);
    impl Window {
        fn new(owner: Option<HWND>, shadow: bool) -> Self {
            static REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            REGISTERED.get_or_init(|| unsafe {
                assert_ne!(
                    RegisterClassW(&WNDCLASSW {
                        lpfnWndProc: Some(window_proc),
                        hInstance: HINSTANCE(GetModuleHandleW(None).unwrap().0),
                        lpszClassName: w!("PotShadowWnd"),
                        ..Default::default()
                    }),
                    0
                );
            });
            Self(unsafe {
                CreateWindowExW(
                    WS_EX_NOACTIVATE,
                    if shadow {
                        w!("PotShadowWnd")
                    } else {
                        w!("STATIC")
                    },
                    w!("Otter decoration fixture"),
                    WS_POPUP,
                    0,
                    0,
                    100,
                    100,
                    owner,
                    None,
                    Some(HINSTANCE(GetModuleHandleW(None).unwrap().0)),
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
    fn excludes_only_shadow_of_an_authorized_popup_not_generic_popup_or_root() {
        let root = Window::new(None, false);
        let menu = Window::new(Some(root.0), false);
        let shadow = Window::new(Some(menu.0), true);
        let tooltip = Window::new(Some(menu.0), false);
        let root_shadow = Window::new(Some(root.0), true);
        let target = Target::bind(root.0).unwrap();
        for (window, expected) in [
            (shadow.0, true),
            (menu.0, false),
            (tooltip.0, false),
            (root_shadow.0, false),
        ] {
            let scope = Scope::bind(&target, None, window).unwrap();
            assert_eq!(is_decoration(&target, None, &scope, &[]), expected);
        }
        let scope = Scope::bind(&target, None, shadow.0).unwrap();
        unsafe {
            SetWindowLongPtrW(shadow.0, GWLP_HWNDPARENT, 0);
        }
        assert!(!is_decoration(&target, None, &scope, &[]));
    }
}
