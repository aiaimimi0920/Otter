//! 同一恢复 journal 的真实子进程登记/退休；只借用自有 fixture 菜单。
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::{pump, Fixture};
use std::time::{Duration, Instant};

fn advance(owner: &mut Owner, done: impl Fn(&Owner) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        pump();
        owner.poll().unwrap();
        if done(owner) {
            return;
        }
        assert!(Instant::now() < deadline, "Popup layer journal timed out");
        // 真实 ACK 来自后台 worker；只泵自有窗口，不抢系统前台。
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn nonmodal_layer_waits_for_ack_and_retires_after_control_revocation() {
    let f = Fixture::new();
    let lease = f.show();
    assert!(!f.topmost(), "source transaction has already returned");
    let mut owner = Owner::fixture();
    owner.sync_popup_layers(&[lease]).unwrap();
    assert!(!f.topmost(), "registration request is not an ACK");
    assert_eq!(value(f.popup.0, MANAGED), 0);
    advance(&mut owner, |_| f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), lease.instance);
    assert!(lease.valid());
    f.owner.control(false).unwrap();
    advance(&mut owner, Owner::finished);
    assert!(!f.topmost());
    assert!(owner.popup_layers.is_empty());
    assert!(owner.journal.is_none());
}
#[test]
fn mirror_removal_close_and_hide_all_restore_and_reap_the_shared_journal() {
    for boundary in 0..3 {
        let f = Fixture::new();
        let lease = f.show();
        let mut owner = Owner::fixture();
        owner.sync_popup_layers(&[lease]).unwrap();
        advance(&mut owner, |_| f.topmost());
        match boundary {
            0 => owner.sync_popup_layers(&[]).unwrap(),
            1 => owner.close(),
            _ => {
                f.hide();
                assert!(
                    !f.topmost(),
                    "source hide releases a managed layer synchronously"
                );
            }
        }
        advance(&mut owner, Owner::finished);
        assert!(!f.topmost());
        assert_eq!(value(f.popup.0, MANAGED), 0);
    }
}
#[test]
fn original_topmost_menu_is_not_registered_and_pending_close_never_borrows() {
    let f = Fixture::new();
    let lease = f.show();
    unsafe {
        SetWindowPos(
            f.popup.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let mut owner = Owner::fixture();
    owner.sync_popup_layers(&[lease]).unwrap();
    assert!(owner.popup_layers.is_empty());
    owner.close();
    advance(&mut owner, Owner::finished);
    assert!(
        f.topmost(),
        "application-owned topmost is not Otter's recovery responsibility"
    );
    unsafe {
        SetWindowPos(
            f.popup.0,
            Some(HWND_NOTOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    let mut owner = Owner::fixture();
    owner.sync_popup_layers(&[lease]).unwrap();
    owner.close();
    advance(&mut owner, Owner::finished);
    assert!(!f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), 0);
}

#[test]
fn rapid_reopen_waits_for_old_registration_retirement_before_borrowing_new_instance() {
    let f = Fixture::new();
    let first = f.show();
    let mut journal = Journal::fixture();
    let mut layers = Layers::default();
    layers.sync(&[first], &mut journal).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !layers.entries[0].ticket.ready() {
        journal.poll();
        assert!(
            Instant::now() < deadline,
            "old registration needs a real ACK"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let second = f.show();
    assert_ne!(first.instance, second.instance);
    layers.sync(&[second], &mut journal).unwrap();
    assert_eq!(
        layers.entries.len(),
        1,
        "old HWND registration is still owned"
    );
    assert!(
        !f.topmost(),
        "neither generation has an acknowledged host borrow"
    );
    while !layers.is_empty() {
        pump();
        journal.poll();
        layers
            .poll(false)
            .unwrap_or_else(|error| assert_eq!(error.code(), RETIREMENT_PENDING));
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(second.valid());
    assert!(!f.topmost());
    layers.sync(&[second], &mut journal).unwrap();
    assert!(!f.topmost(), "new display instance requires its own ACK");
    while !f.topmost() {
        pump();
        journal.poll();
        layers
            .poll(false)
            .unwrap_or_else(|error| assert_eq!(error.code(), RETIREMENT_PENDING));
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(value(f.popup.0, MANAGED), second.instance);
    layers.sync(&[], &mut journal).unwrap();
    while !layers.is_empty() {
        pump();
        journal.poll();
        layers
            .poll(false)
            .unwrap_or_else(|error| assert_eq!(error.code(), RETIREMENT_PENDING));
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!f.topmost());
    journal.close();
    while !journal.finished() {
        journal.poll();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn acknowledged_layered_menu_does_not_require_full_snapshot_and_reaps_on_close() {
    let f = Fixture::new();
    f.layered_without_attributes();
    // per-pixel 样式/内容先于本轮 SHOW，不沿用之前的显示实例。
    let lease = f.show();
    let target = Target::bind(f.popup.0).unwrap();
    assert!(Snapshot::read(&target).is_err());
    let before = unsafe { GetWindowLongPtrW(f.popup.0, GWL_EXSTYLE) };
    assert!(
        lease.valid(),
        "lease changed after snapshot: {lease:?}, current={:?}",
        PopupLease::read(f.popup.0)
    );
    assert!(
        eligible(lease),
        "ineligible layer: popup={before:#x}, root={:#x}, temporary={}, managed={}",
        unsafe { GetWindowLongPtrW(f.root.0, GWL_EXSTYLE) },
        value(f.popup.0, TEMPORARY),
        value(f.popup.0, MANAGED)
    );
    let mut owner = Owner::fixture();
    owner.sync_popup_layers(&[lease]).unwrap();
    assert_eq!(owner.popup_layers.entries.len(), 1);
    assert!(
        !f.topmost(),
        "partial record does not bypass real journal ACK"
    );
    advance(&mut owner, |_| f.topmost());
    assert_eq!(value(f.popup.0, MANAGED), lease.instance);
    owner.close();
    advance(&mut owner, Owner::finished);
    assert_eq!(unsafe { GetWindowLongPtrW(f.popup.0, GWL_EXSTYLE) }, before);
    assert!(Snapshot::read(&target).is_err());
    assert_eq!(value(f.popup.0, MANAGED), 0);
}

#[test]
fn same_lease_becoming_wanted_during_posted_restore_cannot_reborrow_retiring_registration() {
    let f = Fixture::new();
    let lease = f.show();
    let mut owner = Owner::fixture();
    owner.sync_popup_layers(&[lease]).unwrap();
    advance(&mut owner, |_| f.topmost());
    owner.sync_popup_layers(&[]).unwrap();
    owner.poll().unwrap(); // Restore 已入队，还未被源线程 PM_REMOVE 执行。
    assert!(owner.popup_layers.entries[0].retiring);
    assert!(!owner.popup_layers.entries[0].restored);
    assert!(owner.deadline().is_some());
    owner.sync_popup_layers(&[lease]).unwrap();
    assert!(owner.popup_layers.entries[0].wanted);
    assert!(owner.popup_layers.entries[0].retiring);
    advance(&mut owner, Owner::finished);
    assert!(lease.valid());
    assert!(!f.topmost(), "wanted cannot cancel an in-flight retirement");
    assert_eq!(value(f.popup.0, MANAGED), 0);
    assert!(owner.deadline().is_none());
}
