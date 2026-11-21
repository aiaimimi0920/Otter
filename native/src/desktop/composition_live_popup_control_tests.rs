//! 父控制周期继承与子失败恢复：复用自有 Live/帧源，不创建 worker 或运行 WGC。
use super::*;

#[test]
fn a_failed_child_is_not_reenabled_by_repeated_polls_of_the_same_parent_cycle() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent_frames, parent) = live();
    let (_child_frames, child) = live();
    let grant = parent.popup_control_grant().unwrap();
    child.sync_popup_control(Some(grant));
    let own_epoch = child
        .hover_identity()
        .unwrap()
        .capture_control()
        .unwrap()
        .epoch();
    child.sync_popup_control(Some(grant));
    assert_eq!(child.take_input_request(), None);
    assert_eq!(
        child
            .hover_identity()
            .unwrap()
            .capture_control()
            .unwrap()
            .epoch(),
        own_epoch
    );
    child.input_failure();
    assert_eq!(child.take_input_request(), Some(false));
    assert!(child.take_input_notice());
    for _ in 0..4 {
        child.sync_popup_control(Some(grant));
        assert_eq!(child.take_input_request(), None);
        assert!(!child.popup_control_allowed());
        assert!(child.state.input.borrow().failed);
        assert!(!child.state.input.borrow().pending);
        assert!(!child.state.input.borrow().desired);
        assert!(!child.take_input_notice());
        assert_eq!(child.pop_input(), None);
    }
}

#[test]
fn temporarily_missing_parent_grant_does_not_reset_the_failed_cycle_marker() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent_frames, parent) = live();
    let (_child_frames, child) = live();
    let grant = parent.popup_control_grant().unwrap();
    child.sync_popup_control(Some(grant));
    child.input_failure();
    child.take_input_request();
    child.sync_popup_control(None);
    child.sync_popup_control(Some(grant));
    assert!(grant.valid(), "the same parent cycle is still active");
    assert_eq!(child.take_input_request(), None);
    assert!(child.state.input.borrow().failed);
    assert!(!child.state.input.borrow().desired);
}

#[test]
fn a_new_valid_parent_cycle_can_request_fresh_enable_without_replaying_failed_input() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent_frames, parent) = live();
    let (_child_frames, child) = live();
    let old = parent.popup_control_grant().unwrap();
    child.sync_popup_control(Some(old));
    child
        .state
        .input
        .borrow_mut()
        .push(Event::Character { unit: 65 });
    child.input_failure();
    child.take_input_request();
    parent.suspend_source_input();
    assert!(!old.valid());
    assert!(parent.popup_control_grant().is_none());
    child.sync_popup_control(Some(old));
    assert_eq!(child.take_input_request(), None);
    parent.request_default_input();
    parent.take_input_request();
    parent.input_status(false, true, None);
    let renewed = parent.popup_control_grant().unwrap();
    assert_ne!(renewed.epoch(), old.epoch());
    child.sync_popup_control(Some(renewed));
    assert_eq!(child.take_input_request(), Some(true));
    assert_eq!(child.pop_input(), None);
    child.sync_popup_control(Some(renewed));
    assert_eq!(child.take_input_request(), None);
    // 只有原 Session 真正开始下一轮准入时，才按已有状态路径清掉失败标记。
    child.input_status(false, true, None);
    assert!(!child.state.input.borrow().failed);
    assert!(child.popup_control_allowed());
}

#[test]
fn parent_identity_cookie_prevents_reusing_a_failure_marker_for_a_different_owner() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent_frames, parent) = live();
    let (_child_frames, child) = live();
    let old = parent.popup_control_grant().unwrap();
    child.sync_popup_control(Some(old));
    child.input_failure();
    child.take_input_request();
    // 同 HWND/相同首个 epoch 不等于同 Owner；覆盖仅限夹具自有窗口。
    let replacement = crate::live_hover_owner::Owner::new(parent.hwnd).unwrap();
    replacement.control(true).unwrap();
    let current = replacement.identity().capture_control().unwrap();
    assert_eq!(current.epoch(), old.epoch());
    assert_ne!(current.identity().cookie, old.identity().cookie);
    assert!(!old.valid());
    child.sync_popup_control(Some(old));
    assert_eq!(child.take_input_request(), None);
    child.sync_popup_control(Some(current));
    assert_eq!(child.take_input_request(), Some(true));
    assert_eq!(child.pop_input(), None);
}
