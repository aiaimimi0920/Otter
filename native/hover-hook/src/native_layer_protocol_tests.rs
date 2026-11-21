//! 固定源线程层级协议：原输入 fixture 建立真实 lease，独立请求 HWND 不占用 B 输入槽。
use super::*;
use crate::{native_layer_dispatch as layer, native_layer_protocol as p};
use std::cell::Cell;

thread_local! {
    static REQUEST_WINDOW: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static RETAINED_DURING_CHANGE: Cell<bool> = const { Cell::new(false) };
}
struct LayerFixture {
    _hooks: Hooks,
    source: Fixture,
    popup: Window,
    window: Window,
    lease: Lease,
}
impl LayerFixture {
    fn new() -> Self {
        let source = Fixture::new();
        let popup = Window::new(None);
        let window = Window::new(None);
        let hooks = Hooks::new();
        unsafe {
            SetWindowPos(
                source.root.0,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
            .unwrap();
        }
        POSITION_HANDLER.with(|slot| slot.set(None));
        POPUP.with(|slot| slot.set(popup.0));
        BUTTON_HANDLER.with(|slot| {
            slot.set(Some(|| unsafe {
                let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
            }))
        });
        assert!(dispatch::dispatch(source.owner.0, 303, source.child.0).unwrap());
        BUTTON_HANDLER.with(|slot| slot.set(None));
        let lease = Lease::read(popup.0).unwrap();
        assert!(!topmost(popup.0));
        Self {
            _hooks: hooks,
            source,
            popup,
            window,
            lease,
        }
    }
    fn publish(&self, operation: p::Operation, nonce: usize) -> p::Request {
        self.publish_at(self.window.0, operation, nonce)
    }
    fn publish_at(&self, window: HWND, operation: p::Operation, nonce: usize) -> p::Request {
        let lease = self.lease;
        for (key, val) in [
            (p::COMMIT, 0),
            (p::TOKEN, 707),
            (p::ID, nonce),
            (p::OPERATION, operation as usize),
            (p::POPUP, lease.popup.0 as usize),
            (p::OWNER, lease.owner.0 as usize),
            (p::COOKIE, lease.cookie),
            (p::EPOCH, lease.epoch),
            (p::ROOT, lease.root.0 as usize),
            (p::INPUT, lease.request),
            (p::INSTANCE, lease.instance),
            (p::CLAIMED, 0),
            (p::RESULT, 0),
            (p::FINISHED, 0),
            (p::COMMIT, nonce),
        ] {
            put(window, key, val);
        }
        p::Request {
            window,
            token: 707,
            nonce,
            operation,
            lease,
        }
    }
}
impl Drop for LayerFixture {
    fn drop(&mut self) {
        POSITION_HANDLER.with(|slot| slot.set(None));
        BUTTON_HANDLER.with(|slot| slot.set(None));
        REQUEST_WINDOW.with(|slot| slot.set(HWND::default()));
        unsafe {
            let _ = ShowWindow(self.popup.0, SW_HIDE);
        }
        POPUP.with(|slot| slot.set(HWND::default()));
    }
}
fn topmost(hwnd: HWND) -> bool {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
}
fn send(request: p::Request) {
    assert!(layer::dispatch(request.window, request.nonce, request.lease.popup).unwrap());
}

#[test]
fn request_claim_cancel_finish_and_new_identity_are_separate_states() {
    let f = LayerFixture::new();
    let first = f.publish(p::Operation::Acquire, 1);
    assert!(first.active());
    put(f.window.0, p::COMMIT, 0);
    assert!(!first.claim().unwrap());
    assert_eq!(value(f.window.0, p::CLAIMED), 0);
    assert_eq!(first.result(), None);

    let second = f.publish(p::Operation::Acquire, 2);
    assert!(second.claim().unwrap());
    assert!(!second.claim().unwrap());
    put(f.window.0, p::COMMIT, 0);
    assert!(!second.active());
    assert!(
        second.matches(),
        "cancellation cannot erase an in-flight identity"
    );
    assert_eq!(second.result(), None, "claiming is not completion");
    second.finish(E_ABORT).unwrap();
    assert_eq!(second.result(), Some(E_ABORT));

    let third = f.publish(p::Operation::Restore, 3);
    second.finish(S_OK).unwrap();
    assert_eq!(
        third.result(),
        None,
        "late old completion cannot overwrite new request"
    );
    assert_eq!(value(f.window.0, p::FINISHED), 0);
    for (key, bad, good) in [
        (
            p::OPERATION,
            WM_COMMAND as usize,
            p::Operation::Restore as usize,
        ),
        (p::TOKEN, 0, third.token),
        (p::INSTANCE, 0, third.lease.instance),
        (p::ID, 0, third.nonce),
    ] {
        put(f.window.0, key, bad);
        assert!(p::Request::read(f.window.0, 3, f.popup.0).is_none());
        put(f.window.0, key, good);
    }
    assert!(p::Request::read(f.window.0, 3, f.source.root.0).is_none());
    let (window, popup) = (f.window.0 .0 as usize, f.popup.0 .0 as usize);
    assert!(std::thread::spawn(move || {
        p::Request::read(HWND(window as *mut _), 3, HWND(popup as *mut _)).is_none()
    })
    .join()
    .unwrap());
}

#[test]
fn duplicate_layer_hooks_acquire_once_and_restore_without_live_b_epoch() {
    let f = LayerFixture::new();
    let mut hooks = Hooks(Vec::new());
    for _ in 0..2 {
        hooks.0.push(unsafe {
            SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(layer::otter_live_layer_call),
                None,
                GetCurrentThreadId(),
            )
            .unwrap()
        });
    }
    let request = f.publish(p::Operation::Acquire, 1);
    unsafe {
        SendMessageW(
            f.popup.0,
            layer::message_id(),
            Some(WPARAM(f.window.0 .0 as usize)),
            Some(LPARAM(1)),
        );
    }
    assert_eq!(request.result(), Some(S_OK));
    assert!(topmost(f.popup.0));
    assert_eq!(value(f.popup.0, MANAGED), f.lease.instance);
    assert!(!layer::dispatch(f.window.0, 1, f.popup.0).unwrap());
    assert_eq!(
        value(f.source.owner.0, FINISHED),
        303,
        "B input slot is unchanged"
    );
    put(f.source.owner.0, EPOCH, 0);
    assert!(!f.lease.valid());
    assert!(f.lease.same_instance());
    let restore = f.publish(p::Operation::Restore, 2);
    send(restore);
    assert_eq!(restore.result(), Some(S_OK));
    assert!(!topmost(f.popup.0));
    assert_eq!(value(f.popup.0, MANAGED), 0);
}

#[test]
fn acquire_hide_reshow_during_source_call_cannot_let_old_write_escape_to_new_instance() {
    let f = LayerFixture::new();
    let request = f.publish(p::Operation::Acquire, 1);
    RETAINED_DURING_CHANGE.with(|slot| slot.set(false));
    POSITION_HANDLER.with(|slot| {
        slot.set(Some(|hwnd| unsafe {
            if hwnd != POPUP.with(Cell::get) || value(hwnd, MANAGED) == 0 {
                return;
            }
            POSITION_HANDLER.with(|slot| slot.set(None));
            let Some(lease) = Lease::read(hwnd) else {
                return;
            };
            let _ = ShowWindow(hwnd, SW_HIDE);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            RETAINED_DURING_CHANGE.with(|slot| {
                slot.set(lease.same_instance() && !lease.valid() && Lease::read(hwnd).is_none())
            });
        }))
    });
    send(request);
    assert!(RETAINED_DURING_CHANGE.with(Cell::get));
    assert_eq!(request.result(), Some(E_ABORT));
    assert!(!topmost(f.popup.0));
    assert_eq!(value(f.popup.0, MANAGED), 0);
    let restore = f.publish(p::Operation::Restore, 2);
    send(restore);
    assert_eq!(restore.result(), Some(S_OK));
}

#[test]
fn claimed_acquire_cancelled_inside_window_callback_rolls_back_and_still_finishes() {
    let f = LayerFixture::new();
    let request = f.publish(p::Operation::Acquire, 1);
    REQUEST_WINDOW.with(|slot| slot.set(f.window.0));
    POSITION_HANDLER.with(|slot| {
        slot.set(Some(|hwnd| unsafe {
            if hwnd != POPUP.with(Cell::get) || value(hwnd, MANAGED) == 0 {
                return;
            }
            POSITION_HANDLER.with(|slot| slot.set(None));
            let _ = SetPropW(
                REQUEST_WINDOW.with(Cell::get),
                p::COMMIT,
                Some(HANDLE::default()),
            );
        }))
    });
    send(request);
    assert_eq!(value(f.window.0, p::COMMIT), 0);
    assert_eq!(
        request.result(),
        Some(E_ABORT),
        "cancelled claim still needs a terminal receipt"
    );
    assert!(
        f.lease.valid(),
        "layer cancellation is not input authorization revocation"
    );
    assert!(!topmost(f.popup.0));
    assert_eq!(value(f.popup.0, MANAGED), 0);
}

#[test]
fn source_owned_topmost_is_neither_acquired_nor_restored_by_layer_request() {
    let f = LayerFixture::new();
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
    let acquire = f.publish(p::Operation::Acquire, 1);
    send(acquire);
    assert_eq!(acquire.result(), Some(E_ABORT));
    assert_eq!(value(f.popup.0, MANAGED), 0);
    let restore = f.publish(p::Operation::Restore, 2);
    send(restore);
    assert_eq!(restore.result(), Some(S_OK));
    assert!(topmost(f.popup.0));
}

#[test]
fn posted_layer_request_executes_only_on_remove_and_is_not_delivered_twice() {
    let f = LayerFixture::new();
    let mut hooks = Hooks(Vec::new());
    for _ in 0..2 {
        hooks.0.push(unsafe {
            SetWindowsHookExW(
                WH_GETMESSAGE,
                Some(layer::otter_live_layer_get),
                None,
                GetCurrentThreadId(),
            )
            .unwrap()
        });
    }
    let request = f.publish(p::Operation::Acquire, 1);
    let id = layer::message_id();
    assert_ne!(id, 0);
    unsafe {
        PostMessageW(
            Some(f.popup.0),
            id,
            WPARAM(f.window.0 .0 as usize),
            LPARAM(1),
        )
        .unwrap();
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, Some(f.popup.0), id, id, PM_NOREMOVE).as_bool());
        assert_eq!(value(f.window.0, p::CLAIMED), 0);
        assert!(
            !topmost(f.popup.0),
            "peeking does not execute layer operations"
        );
        assert!(PeekMessageW(&mut message, Some(f.popup.0), id, id, PM_REMOVE).as_bool());
        assert_eq!(
            message.message, WM_NULL,
            "claimed private packet is consumed"
        );
        assert_eq!(request.result(), Some(S_OK));
        assert!(topmost(f.popup.0));
        assert!(!layer::dispatch(f.window.0, 1, f.popup.0).unwrap());
        DispatchMessageW(&message);
    }
    let restore = f.publish(p::Operation::Restore, 2);
    send(restore);
    assert_eq!(restore.result(), Some(S_OK));
    assert!(!topmost(f.popup.0));
}

#[test]
fn nested_recovery_cannot_confirm_restored_while_original_acquire_is_still_mutating() {
    let f = LayerFixture::new();
    let recovery_window = Window::new(None);
    let recovery = f.publish_at(recovery_window.0, p::Operation::Restore, 2);
    let acquire = f.publish(p::Operation::Acquire, 1);
    REQUEST_WINDOW.with(|slot| slot.set(recovery_window.0));
    POSITION_HANDLER.with(|slot| {
        slot.set(Some(|hwnd| {
            if hwnd != POPUP.with(Cell::get) || value(hwnd, MANAGED) == 0 {
                return;
            }
            POSITION_HANDLER.with(|slot| slot.set(None));
            let _ = layer::dispatch(REQUEST_WINDOW.with(Cell::get), 2, hwnd);
        }))
    });
    send(acquire);
    assert_eq!(acquire.result(), Some(S_OK));
    assert_eq!(recovery.result(), Some(p::PENDING));
    assert!(
        topmost(f.popup.0),
        "nested recovery did not steal the outer mutation"
    );
    let retry = f.publish_at(recovery_window.0, p::Operation::Restore, 3);
    send(retry);
    assert_eq!(retry.result(), Some(S_OK));
    assert!(!topmost(f.popup.0));
    assert_eq!(value(f.popup.0, MANAGED), 0);
}
