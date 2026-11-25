//! Owned hidden windows verify owner-chain admission and revocation; no user applications or window-title dependencies.
use super::*;
use windows::core::w;

struct Window(HWND);
impl Window {
    fn new(owner: Option<HWND>, child: bool) -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Otter popup scope test"),
                if child { WS_CHILD } else { WS_POPUP },
                0,
                0,
                100,
                100,
                owner,
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
            if IsWindow(Some(self.0)).as_bool() {
                DestroyWindow(self.0).unwrap();
            }
        }
    }
}

#[test]
fn accepts_direct_and_nested_owned_top_level_popups() {
    let root = Window::new(None, false);
    let popup = Window::new(Some(root.0), false);
    let nested = Window::new(Some(popup.0), false);
    let target = Target::bind(root.0).unwrap();
    for hwnd in [popup.0, nested.0] {
        let binding = OwnedPopup::bind(&target, hwnd).unwrap();
        assert_eq!(binding.target().unwrap().hwnd(), hwnd);
    }
}

#[test]
fn rejects_root_children_and_same_process_unrelated_windows() {
    let root = Window::new(None, false);
    let other = Window::new(None, false);
    let other_popup = Window::new(Some(other.0), false);
    let child = Window::new(Some(root.0), true);
    let target = Target::bind(root.0).unwrap();
    for hwnd in [root.0, other.0, other_popup.0, child.0] {
        assert!(OwnedPopup::bind(&target, hwnd).is_err());
    }
}

#[test]
fn revokes_binding_when_owner_changes_or_popup_is_destroyed() {
    let root = Window::new(None, false);
    let other = Window::new(None, false);
    let popup = Window::new(Some(root.0), false);
    let target = Target::bind(root.0).unwrap();
    let binding = OwnedPopup::bind(&target, popup.0).unwrap();
    unsafe {
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, other.0 .0 as isize);
    }
    assert!(binding.target().is_err());
    unsafe {
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, root.0 .0 as isize);
    }
    assert!(binding.validate().is_ok());
    drop(popup);
    assert!(binding.target().is_err());
}

#[test]
fn revokes_nested_binding_when_intermediate_owner_changes() {
    let root = Window::new(None, false);
    let popup = Window::new(Some(root.0), false);
    let nested = Window::new(Some(popup.0), false);
    let binding = OwnedPopup::bind(&Target::bind(root.0).unwrap(), nested.0).unwrap();
    unsafe {
        SetWindowLongPtrW(popup.0, GWLP_HWNDPARENT, 0);
    }
    assert!(binding.validate().is_err());
}

thread_local! {
    static SHOW_POPUP: std::cell::Cell<HWND> = const { std::cell::Cell::new(HWND(std::ptr::null_mut())) };
}

#[test]
fn descendant_scope_is_bounded_and_loses_authority_with_parent_mirror() {
    let root = Window::new(None, false);
    let first = Window::new(Some(root.0), false);
    let second = Window::new(Some(first.0), false);
    let third = Window::new(Some(second.0), false);
    let fourth = Window::new(Some(third.0), false);
    let fifth = Window::new(Some(fourth.0), false);
    let b = Window::new(None, false);
    let owner = crate::live_hover_owner::Owner::new(b.0).unwrap();
    unsafe {
        let _ = ShowWindow(b.0, SW_SHOWNOACTIVATE);
    }
    owner.control(true).unwrap();
    let mut scope = Scope::bind(&Target::bind(root.0).unwrap(), None, first.0).unwrap();
    for hwnd in [second.0, third.0, fourth.0] {
        scope = scope.bind_descendant(Some(owner.identity()), hwnd).unwrap();
        assert_eq!(scope.target().unwrap().hwnd(), hwnd);
    }
    assert!(scope
        .bind_descendant(Some(owner.identity()), fifth.0)
        .is_err());
    owner.control(false).unwrap();
    assert!(scope.target().is_err());
}
unsafe extern "system" fn popup_source(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if message == WM_RBUTTONUP {
        let _ = ShowWindow(SHOW_POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
struct Hooks(Vec<HHOOK>);
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
fn leased_popup_and_owned_descendant_are_bound_to_exact_live_owner_and_epoch() {
    use otter_live_hover_hook::{native_dispatch, native_popup, native_protocol as p};
    let root = Window::new(None, false);
    let popup = Window::new(None, false);
    let descendant = Window::new(Some(popup.0), false);
    let b = Window::new(None, false);
    let b2 = Window::new(None, false);
    let owner = crate::live_hover_owner::Owner::new(b.0).unwrap();
    let other = crate::live_hover_owner::Owner::new(b2.0).unwrap();
    owner.control(true).unwrap();
    other.control(true).unwrap();
    SHOW_POPUP.with(|slot| slot.set(popup.0));
    let target = Target::bind(root.0).unwrap();
    let mut hooks = Hooks(Vec::new());
    unsafe {
        let _ = ShowWindow(b.0, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(b2.0, SW_SHOWNOACTIVATE);
        SetWindowLongPtrW(root.0, GWLP_WNDPROC, popup_source as *const () as isize);
        let thread = GetWindowThreadProcessId(root.0, None);
        hooks.0.push(
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(native_dispatch::otter_live_native_call),
                None,
                thread,
            )
            .unwrap(),
        );
        hooks.0.push(
            SetWindowsHookExW(
                WH_CALLWNDPROCRET,
                Some(native_popup::otter_live_native_return),
                None,
                thread,
            )
            .unwrap(),
        );
        otter_live_hover_hook::native_input_scope::Snapshot::default()
            .publish(b.0)
            .unwrap();
        for (key, value) in [
            (p::COOKIE, owner.identity().cookie),
            (p::REQUEST_EPOCH, p::value(b.0, p::EPOCH)),
            (p::ROOT, root.0 .0 as usize),
            (p::DESTINATION, root.0 .0 as usize),
            (p::MESSAGE, WM_RBUTTONUP as usize),
            (p::WP, 0),
            (p::LP, 0),
            (p::COMMIT, 303),
        ] {
            SetPropW(b.0, key, Some(HANDLE(value as *mut _))).unwrap();
        }
        SendMessageW(
            root.0,
            native_dispatch::message_id(),
            Some(WPARAM(b.0 .0 as usize)),
            Some(LPARAM(303)),
        );
    }
    assert!(OwnedPopup::bind(&target, popup.0).is_err());
    let scope = Scope::bind(&target, Some(owner.identity()), popup.0).unwrap();
    let nested = Scope::bind(&target, Some(owner.identity()), descendant.0).unwrap();
    assert!(scope.target().is_ok());
    assert!(nested.target().is_ok());
    assert!(Scope::bind(&target, None, popup.0).is_err());
    assert!(Scope::bind(&target, Some(other.identity()), popup.0).is_err());
    // The second ownerless popup belongs to the first-level popup's independent B and cannot impersonate the main B's lease.
    let second = Window::new(None, false);
    SHOW_POPUP.with(|slot| slot.set(second.0));
    otter_live_hover_hook::native_input_scope::Snapshot::default()
        .publish(b2.0)
        .unwrap();
    unsafe {
        SetWindowLongPtrW(popup.0, GWLP_WNDPROC, popup_source as *const () as isize);
        for (key, value) in [
            (p::COOKIE, other.identity().cookie),
            (p::REQUEST_EPOCH, p::value(b2.0, p::EPOCH)),
            (p::ROOT, popup.0 .0 as usize),
            (p::DESTINATION, popup.0 .0 as usize),
            (p::MESSAGE, WM_RBUTTONUP as usize),
            (p::WP, 0),
            (p::LP, 0),
            (p::COMMIT, 304),
        ] {
            SetPropW(b2.0, key, Some(HANDLE(value as *mut _))).unwrap();
        }
        SendMessageW(
            popup.0,
            native_dispatch::message_id(),
            Some(WPARAM(b2.0 .0 as usize)),
            Some(LPARAM(304)),
        );
    }
    assert!(Scope::bind(&target, Some(owner.identity()), second.0).is_err());
    assert!(scope
        .bind_descendant(Some(owner.identity()), second.0)
        .is_err());
    let chained = scope
        .bind_descendant(Some(other.identity()), second.0)
        .unwrap();
    assert_eq!(chained.target().unwrap().hwnd(), second.0);
    assert_eq!(chained.mirror_owner(), Some(b2.0));
    owner.control(false).unwrap();
    assert!(scope.target().is_err());
    assert!(nested.target().is_err());
    assert!(
        chained.target().is_err(),
        "revoking the original B must revoke every child lease"
    );
    owner.control(true).unwrap();
    assert!(scope.target().is_err());
    assert!(Scope::bind(&target, Some(owner.identity()), popup.0).is_err());
}
