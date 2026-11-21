//! 已执行操作的完成不是新授权；自有源窗口的销毁/撤销不能确认新请求或恢复弹窗来源。
use super::*;
use crate::native_dispatch as dispatch;
use std::cell::Cell;

struct Handler;
impl Handler {
    fn set(callback: fn()) -> Self {
        BUTTONS.with(|count| count.set(0));
        BUTTON_HANDLER.with(|slot| slot.set(Some(callback)));
        Self
    }
}
impl Drop for Handler {
    fn drop(&mut self) {
        BUTTON_HANDLER.with(|slot| slot.set(None));
    }
}

#[test]
fn callback_destroying_its_destination_still_confirms_exactly_one_execution() {
    let f = Fixture::new();
    let _handler = Handler::set(|| unsafe {
        let request = dispatch::collecting().unwrap();
        DestroyWindow(request.destination).unwrap();
        assert!(dispatch::collecting().is_none());
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert_eq!(BUTTONS.with(Cell::get), 1);
    assert!(!dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(BUTTONS.with(Cell::get), 1);
}

#[test]
fn callback_destroying_source_root_does_not_erase_the_returned_operation() {
    let f = Fixture::new();
    let _handler = Handler::set(|| unsafe {
        let request = dispatch::collecting().unwrap();
        DestroyWindow(request.root).unwrap();
        assert!(!IsWindow(Some(request.destination)).as_bool());
        assert!(dispatch::collecting().is_none());
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert!(f.read().is_none());
    assert_eq!(BUTTONS.with(Cell::get), 1);
}

#[test]
fn owner_hidden_during_callback_can_confirm_but_cannot_accept_the_next_request() {
    let f = Fixture::new();
    let handler = Handler::set(|| unsafe {
        let request = dispatch::collecting().unwrap();
        let _ = ShowWindow(request.owner, SW_HIDE);
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
    drop(handler);
    f.publish(304);
    assert!(!dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 0);
    assert_eq!(BUTTONS.with(Cell::get), 1);
    unsafe {
        let _ = ShowWindow(f.owner.0, SW_SHOWNOACTIVATE);
    }
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 304);
    assert_eq!(BUTTONS.with(Cell::get), 2);
}

#[test]
fn explicit_publication_cancellation_during_callback_does_not_receive_completion() {
    let f = Fixture::new();
    let _handler = Handler::set(|| {
        let request = dispatch::collecting().unwrap();
        put(request.owner, COMMIT, 0);
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, CLAIMED), 303);
    assert_eq!(value(f.owner.0, FINISHED), 0);
    assert_eq!(BUTTONS.with(Cell::get), 1);
}

#[test]
fn completion_requires_the_original_claim_cookie_and_entire_packet() {
    let f = Fixture::new();
    let request = f.read().unwrap();
    assert!(
        !request.finish_returned().unwrap(),
        "unclaimed request is not a completed callback"
    );
    assert!(request.claim().unwrap());
    for (key, changed, original) in [
        (CLAIMED, 0, 303),
        (COMMIT, 304, 303),
        (COOKIE, 102, 101),
        (scope::COOKIE, 102, 101),
        (REQUEST_EPOCH, 203, 202),
        (ROOT, f.child.0 .0 as usize, f.root.0 .0 as usize),
        (DESTINATION, f.root.0 .0 as usize, f.child.0 .0 as usize),
        (MESSAGE, WM_LBUTTONUP as usize, WM_RBUTTONUP as usize),
        (WP, 1, 0),
        (LP, 42, 0xffff0001),
    ] {
        put(f.owner.0, key, changed);
        assert!(!request.finish_returned().unwrap());
        assert_eq!(value(f.owner.0, FINISHED), 0);
        put(f.owner.0, key, original);
    }
    assert!(request.finish_returned().unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
}

#[test]
fn old_callback_cannot_confirm_a_new_nonce_or_replacement_identity() {
    let f = Fixture::new();
    let old = f.read().unwrap();
    assert!(old.claim().unwrap());
    f.publish(304);
    assert!(!old.finish_returned().unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 0);
    let current = Request::read(f.owner.0, 304, f.child.0).unwrap();
    assert!(current.claim().unwrap());
    assert!(current.finish_returned().unwrap());
    assert!(!old.finish_returned().unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 304);
    // 即使复用同一数字 nonce，换掉 B cookie 也不是旧回调的 publication。
    f.publish(303);
    put(f.owner.0, scope::COOKIE, 102);
    put(f.owner.0, COOKIE, 102);
    let replacement = f.read().unwrap();
    assert!(replacement.claim().unwrap());
    assert!(!old.finish_returned().unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 0);
}

#[test]
fn completing_a_revoked_callback_does_not_grant_a_late_popup_lease() {
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    POPUP.with(|slot| slot.set(popup.0));
    let _handler = Handler::set(|| unsafe {
        let request = dispatch::collecting().unwrap();
        put(request.owner, EPOCH, 0);
        let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
        assert!(dispatch::collecting().is_none());
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert!(crate::native_popup::Lease::read(popup.0).is_none());
    assert!(!dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
}
