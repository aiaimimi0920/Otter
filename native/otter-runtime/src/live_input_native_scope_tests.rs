//! Original-source publication, source-execution rejection, and property cleanup for production Publication; no product DLL/external-application loading.
use super::*;
use crate::live_window_target::{popup::Scope, recovery::popup_layer::fixture::Fixture};
use windows::core::w;

struct B {
    window: HWND,
    owner: crate::live_hover_owner::Owner,
}
impl B {
    fn new() -> Self {
        let window = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                -30000,
                -30000,
                20,
                20,
                None,
                None,
                None,
                None,
            )
        }
        .unwrap();
        let owner = crate::live_hover_owner::Owner::new(window).unwrap();
        unsafe {
            let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
        }
        owner.control(true).unwrap();
        Self { window, owner }
    }
}
impl Drop for B {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.window).unwrap();
        }
    }
}
fn scoped(
    f: &Fixture,
) -> (
    Destination,
    otter_live_hover_hook::native_input_scope::Snapshot,
) {
    let scope = Scope::bind(
        &Target::bind(f.root.0).unwrap(),
        Some(f.owner.identity()),
        f.popup.0,
    )
    .unwrap();
    let root = scope.target().unwrap();
    (
        Destination::bind(&root, root.hwnd()).unwrap(),
        scope.native_snapshot().unwrap(),
    )
}

#[test]
fn source_rejects_a_published_old_display_without_touching_the_new_one() {
    let f = Fixture::new();
    f.show();
    let b = B::new();
    let (target, scope) = scoped(&f);
    let publication = Publication::publish(
        b.owner.identity().capture_control().unwrap(),
        &target,
        Message::motion([2, 3], 0).unwrap(),
        601,
        scope,
    )
    .unwrap();
    assert_eq!(
        protocol::Request::read(b.window, 601, target.hwnd())
            .unwrap()
            .popup_scope,
        scope
    );
    f.show();
    let wire = publication.message().unwrap();
    unsafe {
        SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp));
    }
    assert!(!publication.finished());
    assert_eq!(protocol::value(b.window, protocol::CLAIMED), 0);
    assert!(matches!(
        Publication::publish(
            b.owner.identity().capture_control().unwrap(),
            &target,
            Message::motion([2, 3], 0).unwrap(),
            602,
            scope
        ),
        Err(Error::SourceTransition)
    ));
    assert_eq!(protocol::value(b.window, protocol::COMMIT), 601);
    drop(publication);
    for key in protocol::request_keys() {
        assert_eq!(protocol::value(b.window, key), 0);
    }
}

#[test]
fn current_scoped_operation_finishes_and_old_drop_does_not_clear_the_next_scope() {
    let f = Fixture::new();
    f.show();
    let b = B::new();
    let (target, scope) = scoped(&f);
    let control = b.owner.identity().capture_control().unwrap();
    let old = Publication::publish(
        control,
        &target,
        Message::motion([2, 3], 0).unwrap(),
        603,
        scope,
    )
    .unwrap();
    let current = Publication::publish(
        control,
        &target,
        Message::motion([3, 4], 0).unwrap(),
        604,
        scope,
    )
    .unwrap();
    drop(old);
    assert_eq!(
        protocol::Request::read(b.window, 604, target.hwnd())
            .unwrap()
            .popup_scope,
        scope
    );
    let wire = current.message().unwrap();
    unsafe {
        SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp));
    }
    assert!(current.finished());
    f.owner.control(false).unwrap();
    assert!(
        current.finished(),
        "completion cannot require the original scope to stay authorized"
    );
    drop(current);
    for key in protocol::request_keys() {
        assert_eq!(protocol::value(b.window, key), 0);
    }
}
