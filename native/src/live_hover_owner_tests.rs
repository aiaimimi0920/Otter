//! 只创建自有隐藏 HWND，验证 nonce/撤销/析构，不移动鼠标或向其他程序发消息。
use super::*;
use windows::core::w;
struct Window(HWND);
impl Window {
    fn new() -> Self {
        Self(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                20,
                20,
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
fn hover_owner_active_pause_and_drop_clear_properties() {
    let window = Window::new();
    let owner = Owner::new(window.0).unwrap();
    let id = owner.identity();
    assert!(id.valid());
    owner.active(true).unwrap();
    assert_eq!(unsafe { GetPropW(window.0, ACTIVE) }, id.token());
    owner.active(false).unwrap();
    assert!(unsafe { GetPropW(window.0, ACTIVE) }.is_invalid());
    drop(owner);
    assert!(!id.valid());
    assert!(unsafe { GetPropW(window.0, COOKIE) }.is_invalid());
}
#[test]
fn old_hover_owner_cannot_modify_replacement_identity() {
    let window = Window::new();
    let old = Owner::new(window.0).unwrap();
    let replacement = Owner::new(window.0).unwrap();
    let id = replacement.identity();
    assert!(!old.identity().valid());
    replacement.active(true).unwrap();
    old.active(false).unwrap();
    drop(old);
    assert!(id.valid());
    assert_eq!(unsafe { GetPropW(window.0, ACTIVE) }, id.token());
}
#[test]
fn destroyed_hover_owner_does_not_remain_valid() {
    let window = Window::new();
    let owner = Owner::new(window.0).unwrap();
    drop(window);
    assert!(!owner.identity().valid());
    owner.active(false).unwrap();
}

#[test]
fn control_epoch_survives_leave_but_changes_after_pause_and_clears_on_drop() {
    let window = Window::new();
    let owner = Owner::new(window.0).unwrap();
    owner.control(true).unwrap();
    let first = native_protocol::value(window.0, native_protocol::EPOCH);
    assert_ne!(first, 0);
    owner.active(false).unwrap();
    owner.control(true).unwrap();
    assert_eq!(
        native_protocol::value(window.0, native_protocol::EPOCH),
        first
    );
    owner.control(false).unwrap();
    assert_eq!(native_protocol::value(window.0, native_protocol::EPOCH), 0);
    owner.control(true).unwrap();
    assert_ne!(
        native_protocol::value(window.0, native_protocol::EPOCH),
        first
    );
    drop(owner);
    assert_eq!(native_protocol::value(window.0, native_protocol::EPOCH), 0);
}

#[test]
fn control_snapshot_is_not_reissued_after_pause_or_identity_replacement() {
    let window = Window::new();
    let owner = Owner::new(window.0).unwrap();
    let id = owner.identity();
    assert!(id.capture_control().is_err());
    owner.control(true).unwrap();
    let first = id.capture_control().unwrap();
    assert!(first.valid());
    owner.active(false).unwrap();
    owner.control(true).unwrap();
    assert!(first.valid(), "Leave must not revoke popup control");
    owner.control(false).unwrap();
    assert!(!first.valid());
    assert!(id.capture_control().is_err());
    owner.control(true).unwrap();
    let second = id.capture_control().unwrap();
    assert_ne!(first.epoch(), second.epoch());
    assert!(!first.valid());
    assert!(second.valid());
    let replacement = Owner::new(window.0).unwrap();
    replacement.control(true).unwrap();
    assert!(
        !second.valid(),
        "same numeric epoch is not the same identity"
    );
    assert!(id.capture_control().is_err());
}
