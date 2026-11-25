//! 部分恢复不重放位置/可见性，不要求仍有 B 控制资格，不触碰另一显示实例。
use super::*;
use fixture::{settle, Fixture};
#[test]
fn layer_restore_after_epoch_revocation_preserves_current_geometry_style_and_visibility() {
    let f = Fixture::new();
    let lease = f.show();
    let record = f.record(lease);
    let bound = Binding::bind(&record).unwrap();
    f.borrow(lease);
    f.owner.control(false).unwrap();
    unsafe {
        SetWindowPos(
            f.popup.0,
            None,
            -29000,
            -29000,
            120,
            90,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
        let style = GetWindowLongPtrW(f.popup.0, GWL_EXSTYLE);
        SetWindowLongPtrW(f.popup.0, GWL_EXSTYLE, style | WS_EX_TRANSPARENT.0 as isize);
    }
    let before = Snapshot::read(&Target::bind(f.popup.0).unwrap()).unwrap();
    settle(|| bound.restore_or_retire());
    let after = Snapshot::read(&Target::bind(f.popup.0).unwrap()).unwrap();
    assert!(!f.topmost());
    assert_eq!(before.rect, after.rect);
    assert_eq!(before.visible, after.visible);
    assert_eq!(
        after.extended_style,
        before.extended_style & !(WS_EX_TOPMOST.0 as isize)
    );
    assert_eq!(value(f.popup.0, MANAGED), 0);
}
#[test]
fn no_borrow_or_new_display_instance_cannot_restore_an_unrelated_topmost_value() {
    let f = Fixture::new();
    let first = f.show();
    let bound = Binding::bind(&f.record(first)).unwrap();
    // 尚未完成接管的记录不能撤回源应用本身的值。
    f.popup.set_topmost(true);
    settle(|| bound.restore_or_retire());
    assert!(f.topmost());
    f.popup.set_topmost(false);
    let second = f.show();
    assert_ne!(first.instance, second.instance);
    f.borrow(second);
    settle(|| bound.restore_or_retire());
    assert!(f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), second.instance);
    let mut wrong = f.record(second);
    let Restoration::PopupLayer(layer) = &mut wrong.restoration else {
        panic!("layer fixture")
    };
    layer.instance += 1;
    assert!(Binding::bind(&wrong).is_err());
    f.hide();
    assert!(!f.topmost());
}

#[test]
fn layer_only_record_and_binding_do_not_need_readable_window_snapshot() {
    let f = Fixture::new();
    f.layered_without_attributes();
    let lease = f.show();
    let target = Target::bind(f.popup.0).unwrap();
    assert!(
        Snapshot::read(&target).is_err(),
        "full snapshot must still refuse unknown attributes"
    );
    let record = f.record(lease);
    assert!(matches!(&record.restoration, Restoration::PopupLayer(_)));
    let bound = Binding::bind(&record).unwrap();
    let before = unsafe { GetWindowLongPtrW(f.popup.0, GWL_EXSTYLE) };
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(f.popup.0, &mut rect).unwrap();
    }
    assert!(
        lease.valid(),
        "lease changed before acquire: {lease:?}, current={:?}",
        otter_live_hover_hook::native_popup::Lease::read(f.popup.0)
    );
    f.borrow(lease);
    settle(|| bound.restore_or_retire());
    assert!(!f.topmost());
    assert_eq!(unsafe { GetWindowLongPtrW(f.popup.0, GWL_EXSTYLE) }, before);
    let mut after = RECT::default();
    unsafe {
        GetWindowRect(f.popup.0, &mut after).unwrap();
    }
    assert_eq!(after, rect);
    assert!(unsafe { IsWindowVisible(f.popup.0) }.as_bool());
    assert!(
        Snapshot::read(&target).is_err(),
        "layer recovery must not synthesize layered attributes"
    );
}

#[test]
fn helper_drains_bounded_own_completion_notices_without_consuming_source_messages() {
    let f = Fixture::new();
    let lease = f.show();
    let recovery = Recovery::bind(Target::bind(f.popup.0).unwrap(), &wire(lease)).unwrap();
    assert_eq!(
        recovery.acquire().unwrap_err().code(),
        otter_live_hover_hook::native_layer_protocol::PENDING
    );
    let window = recovery.channel.borrow().notification_window();
    unsafe {
        let id = otter_live_hover_hook::native_layer_dispatch::message_id();
        let mut message = MSG::default();
        // 仅移除源请求，保留 hook 真实 FINISHED 所投递的完成通知供下方验证。
        assert!(PeekMessageW(&mut message, Some(f.popup.0), id, id, PM_REMOVE).as_bool());
        recovery.acquire().unwrap();
        assert!(f.topmost());
        for _ in 0..7 {
            PostMessageW(Some(window), WM_NULL, WPARAM(0), LPARAM(0)).unwrap();
        }
        let marker = WM_APP + 98;
        PostMessageW(Some(f.root.0), marker, WPARAM(0), LPARAM(0)).unwrap();
        recovery.drain_helper_notifications();
        let mut remaining = 0;
        while PeekMessageW(&mut message, Some(window), 0, 0, PM_REMOVE).as_bool() {
            assert_eq!(message.message, WM_NULL);
            remaining += 1;
            assert!(remaining <= 8, "test queue must remain bounded");
        }
        assert_eq!(
            remaining, 4,
            "one real completion plus seven queued notices, four consumed"
        );
        assert!(PeekMessageW(&mut message, Some(f.root.0), marker, marker, PM_NOREMOVE).as_bool());
    }
    settle(|| recovery.restore());
    assert!(!f.topmost());
}
