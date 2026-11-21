//! 宿主来源展平保留原父链、TID 和 lease 实例；只使用自有源 hook/窗口。
use super::*;
use crate::live_window_target::recovery::popup_layer::fixture::Fixture;

struct Window(HWND);
impl Window {
    fn new(owner: Option<HWND>) -> Self {
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                windows::core::w!("STATIC"),
                windows::core::w!(""),
                WS_POPUP,
                -30000,
                -30000,
                20,
                20,
                owner,
                None,
                None,
                None,
            )
        }
        .unwrap();
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Self(hwnd)
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
fn scope(f: &Fixture, hwnd: HWND) -> Scope {
    Scope::bind(
        &Target::bind(f.root.0).unwrap(),
        Some(f.owner.identity()),
        hwnd,
    )
    .unwrap()
}
fn link(root: HWND, target: HWND) -> Link {
    Link {
        root: root.0 as usize,
        root_thread: unsafe { GetWindowThreadProcessId(root, None) } as usize,
        target: target.0 as usize,
        target_thread: unsafe { GetWindowThreadProcessId(target, None) } as usize,
        ..Link::default()
    }
}

#[test]
fn transferred_chained_scope_preserves_both_original_epochs_and_all_lease_fields() {
    let f = Fixture::new();
    let lease = f.show();
    let middle_b = Window::new(None);
    let child = Window::new(Some(f.popup.0));
    let owner = crate::live_hover_owner::Owner::new(middle_b.0).unwrap();
    owner.control(true).unwrap();
    let scope = scope(&f, f.popup.0)
        .bind_descendant(Some(owner.identity()), child.0)
        .unwrap();
    let snapshot = scope.transfer().into_scope().native_snapshot().unwrap();
    let mut expected = Snapshot::default();
    assert!(expected.push(Link {
        owner: lease.owner.0 as usize,
        cookie: lease.cookie,
        epoch: lease.epoch,
        anchor: lease.popup.0 as usize,
        anchor_thread: unsafe { GetWindowThreadProcessId(lease.popup, None) } as usize,
        request: lease.request,
        instance: lease.instance,
        ..link(f.root.0, f.popup.0)
    }));
    assert!(expected.push(Link {
        owner: owner.identity().hwnd,
        cookie: owner.identity().cookie,
        epoch: owner.identity().capture_control().unwrap().epoch(),
        ..link(f.popup.0, child.0)
    }));
    assert_eq!(snapshot, expected);
    assert!(snapshot.valid_for(child.0, middle_b.0));
    owner.control(false).unwrap();
    owner.control(true).unwrap();
    assert!(!snapshot.valid_for(child.0, middle_b.0));
    assert!(
        scope.native_snapshot().is_err(),
        "do not refresh a revoked parent epoch"
    );
}

#[test]
fn a_leased_owned_descendant_preserves_the_original_anchor_not_just_its_final_hwnd() {
    let f = Fixture::new();
    f.show();
    let child = Window::new(Some(f.popup.0));
    let scope = scope(&f, child.0);
    let snapshot = scope.native_snapshot().unwrap();
    assert!(snapshot.valid_for(child.0, f.b.0));
    unsafe {
        SetWindowLongPtrW(child.0, GWLP_HWNDPARENT, 0);
    }
    assert!(!snapshot.valid_for(child.0, f.b.0));
    unsafe {
        SetWindowLongPtrW(child.0, GWLP_HWNDPARENT, f.popup.0 .0 as isize);
    }
    assert!(snapshot.valid_for(child.0, f.b.0));
    f.show();
    assert!(!snapshot.valid_for(child.0, f.b.0));
    assert!(
        scope.native_snapshot().is_err(),
        "never bind the new lease during export"
    );
}

#[test]
fn owned_snapshot_keeps_the_original_root_and_rejects_a_broken_owner_chain() {
    let f = Fixture::new();
    let child = Window::new(Some(f.root.0));
    let scope = scope(&f, child.0);
    let snapshot = scope.native_snapshot().unwrap();
    let mut expected = Snapshot::default();
    assert!(expected.push(link(f.root.0, child.0)));
    assert_eq!(snapshot, expected);
    assert!(snapshot.valid_for(child.0, f.b.0));
    assert!(!snapshot.valid_for(f.root.0, f.b.0));
    unsafe {
        SetWindowLongPtrW(child.0, GWLP_HWNDPARENT, 0);
    }
    assert!(!snapshot.valid_for(child.0, f.b.0));
}
