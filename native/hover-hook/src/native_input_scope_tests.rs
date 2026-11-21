//! 原来源随精确输入进入源线程；复用自有窗口/真实 hook，不运行用户应用。
use super::*;
use crate::{
    native_dispatch as dispatch,
    native_input_scope::{Link, Snapshot},
    native_popup::Lease,
};
use std::cell::Cell;

struct Handler;
impl Handler {
    fn set(callback: Option<fn()>) -> Self {
        BUTTONS.with(|count| count.set(0));
        BUTTON_HANDLER.with(|slot| slot.set(callback));
        Self
    }
}
impl Drop for Handler {
    fn drop(&mut self) {
        BUTTON_HANDLER.with(|slot| slot.set(None));
    }
}
fn owned(root: HWND, target: HWND) -> Link {
    Link {
        root: root.0 as usize,
        root_thread: unsafe { GetWindowThreadProcessId(root, None) } as usize,
        target: target.0 as usize,
        target_thread: unsafe { GetWindowThreadProcessId(target, None) } as usize,
        ..Link::default()
    }
}
fn leased(lease: Lease) -> Link {
    Link {
        owner: lease.owner.0 as usize,
        cookie: lease.cookie,
        epoch: lease.epoch,
        anchor: lease.popup.0 as usize,
        anchor_thread: unsafe { GetWindowThreadProcessId(lease.popup, None) } as usize,
        request: lease.request,
        instance: lease.instance,
        ..owned(lease.root, lease.popup)
    }
}
fn snapshot(links: &[Link]) -> Snapshot {
    let mut scope = Snapshot::default();
    for link in links {
        assert!(scope.push(*link));
    }
    scope
}
fn publish(f: &Fixture, scope: Snapshot, root: HWND, destination: HWND) {
    put(f.owner.0, COMMIT, 0);
    put(f.owner.0, ROOT, root.0 as usize);
    put(f.owner.0, DESTINATION, destination.0 as usize);
    put(f.owner.0, CLAIMED, 0);
    put(f.owner.0, FINISHED, 0);
    scope.publish(f.owner.0).unwrap();
    put(f.owner.0, COMMIT, 303);
}
fn show(f: &Fixture, popup: HWND) -> Lease {
    unsafe {
        let _ = ShowWindow(popup, SW_HIDE);
    }
    f.publish(304);
    POPUP.with(|slot| slot.set(popup));
    let _handler = Handler::set(Some(|| unsafe {
        let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
    }));
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    Lease::read(popup).unwrap()
}

#[test]
fn owned_chain_is_rechecked_after_publication_before_source_execution() {
    let f = Fixture::new();
    let origin = Window::new(None);
    unsafe {
        SetWindowLongPtrW(f.root.0, GWLP_HWNDPARENT, origin.0 .0 as isize);
        let _ = ShowWindow(f.root.0, SW_SHOWNOACTIVATE);
    }
    let scope = snapshot(&[owned(origin.0, f.root.0)]);
    publish(&f, scope, f.root.0, f.child.0);
    let original = f.read().unwrap();
    unsafe {
        SetWindowLongPtrW(f.root.0, GWLP_HWNDPARENT, 0);
    }
    let _handler = Handler::set(None);
    assert!(!original.claim().unwrap());
    assert!(!dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(BUTTONS.with(Cell::get), 0);
    assert_eq!(value(f.owner.0, FINISHED), 0);
}

#[test]
fn same_hwnd_new_lease_cannot_consume_the_previous_display_request() {
    let parent = Fixture::new();
    let input = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    let first = show(&parent, popup.0);
    publish(&input, snapshot(&[leased(first)]), popup.0, popup.0);
    let original = Request::read(input.owner.0, 303, popup.0).unwrap();
    let second = show(&parent, popup.0);
    assert_ne!(first.instance, second.instance);
    let _handler = Handler::set(None);
    assert!(!original.claim().unwrap());
    assert!(!dispatch::dispatch(input.owner.0, 303, popup.0).unwrap());
    assert_eq!(BUTTONS.with(Cell::get), 0);
    publish(&input, snapshot(&[leased(second)]), popup.0, popup.0);
    assert!(dispatch::dispatch(input.owner.0, 303, popup.0).unwrap());
    assert_eq!(BUTTONS.with(Cell::get), 1);
}

#[test]
fn every_original_parent_control_epoch_and_chain_link_must_still_match() {
    let parent = Fixture::new();
    let middle = Fixture::new();
    let input = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    let first = show(&parent, popup.0);
    unsafe {
        SetWindowLongPtrW(input.root.0, GWLP_HWNDPARENT, popup.0 .0 as isize);
        let _ = ShowWindow(input.root.0, SW_SHOWNOACTIVATE);
    }
    let second = Link {
        owner: middle.owner.0 .0 as usize,
        cookie: 101,
        epoch: 202,
        ..owned(popup.0, input.root.0)
    };
    let scope = snapshot(&[leased(first), second]);
    publish(&input, scope, input.root.0, input.child.0);
    assert!(input.read().is_some());
    let _handler = Handler::set(None);
    for owner in [parent.owner.0, middle.owner.0] {
        for epoch in [0, 203] {
            put(owner, EPOCH, epoch);
            assert!(!dispatch::dispatch(input.owner.0, 303, input.child.0).unwrap());
            put(owner, EPOCH, 202);
        }
    }
    let disconnected = Link {
        root: parent.root.0 .0 as usize,
        root_thread: unsafe { GetWindowThreadProcessId(parent.root.0, None) } as usize,
        ..second
    };
    publish(
        &input,
        snapshot(&[leased(first), disconnected]),
        input.root.0,
        input.child.0,
    );
    assert!(
        input.read().is_none(),
        "each child must start at the original parent target"
    );
    assert_eq!(BUTTONS.with(Cell::get), 0);
    publish(&input, scope, input.root.0, input.child.0);
    assert!(dispatch::dispatch(input.owner.0, 303, input.child.0).unwrap());
}

#[test]
fn missing_oversized_or_wrong_thread_scope_never_becomes_an_unscoped_request() {
    let f = Fixture::new();
    let origin = Window::new(None);
    unsafe {
        SetWindowLongPtrW(f.root.0, GWLP_HWNDPARENT, origin.0 .0 as isize);
        let _ = ShowWindow(f.root.0, SW_SHOWNOACTIVATE);
    }
    let link = owned(origin.0, f.root.0);
    let scope = snapshot(&[link]);
    publish(&f, scope, f.root.0, f.child.0);
    let count = crate::native_input_scope::keys().next().unwrap();
    for invalid in [0, crate::native_input_scope::LIMIT + 2, usize::MAX] {
        put(f.owner.0, count, invalid);
        assert!(f.read().is_none());
    }
    publish(
        &f,
        snapshot(&[Link {
            target_thread: 0,
            ..link
        }]),
        f.root.0,
        f.child.0,
    );
    assert!(f.read().is_none());
    publish(&f, scope, f.root.0, f.child.0);
    assert!(f.read().is_some());
    let mut bounded = Snapshot::default();
    for _ in 0..crate::native_input_scope::LIMIT {
        assert!(bounded.push(link));
    }
    assert!(!bounded.push(link));
}

#[test]
fn changed_scope_is_part_of_the_original_completion_identity() {
    let f = Fixture::new();
    let origin = Window::new(None);
    unsafe {
        SetWindowLongPtrW(f.root.0, GWLP_HWNDPARENT, origin.0 .0 as isize);
        let _ = ShowWindow(f.root.0, SW_SHOWNOACTIVATE);
    }
    publish(
        &f,
        snapshot(&[owned(origin.0, f.root.0)]),
        f.root.0,
        f.child.0,
    );
    let original = f.read().unwrap();
    assert!(original.claim().unwrap());
    Snapshot::default().publish(f.owner.0).unwrap();
    assert!(!original.finish_returned().unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 0);
}

#[test]
fn returned_scoped_operation_is_confirmed_even_if_it_hides_its_source() {
    let f = Fixture::new();
    let origin = Window::new(None);
    unsafe {
        SetWindowLongPtrW(f.root.0, GWLP_HWNDPARENT, origin.0 .0 as isize);
        let _ = ShowWindow(f.root.0, SW_SHOWNOACTIVATE);
    }
    publish(
        &f,
        snapshot(&[owned(origin.0, f.root.0)]),
        f.root.0,
        f.child.0,
    );
    let _handler = Handler::set(Some(|| unsafe {
        let request = dispatch::collecting().unwrap();
        let _ = ShowWindow(request.root, SW_HIDE);
        assert!(dispatch::collecting().is_none());
    }));
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert_eq!(BUTTONS.with(Cell::get), 1);
    assert!(f.read().is_none());
}
