//! System session-end admission; query only within Application's read-only wait scope, never treating ordinary window closing as system exit.
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use windows::Win32::{Foundation::*, System::Shutdown::*, UI::WindowsAndMessaging::*};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Running,
    Queried,
    Ended,
}
#[derive(Default)]
pub(super) struct Control(Cell<Phase>);
impl Control {
    pub(super) fn paused(&self) -> bool {
        self.0.get() != Phase::Running
    }
    pub(super) fn ended(&self) -> bool {
        self.0.get() == Phase::Ended
    }
}

#[derive(Clone, Copy)]
struct Query {
    owner: *const Control,
    context: *const (),
    call: unsafe fn(*const ()) -> bool,
}
thread_local! {
    static QUERY: Cell<Option<Query>> = const { Cell::new(None) };
    static OWNER: RefCell<Weak<Control>> = const { RefCell::new(Weak::new()) };
}
struct OwnerScope(Weak<Control>);
impl Drop for OwnerScope {
    fn drop(&mut self) {
        OWNER.with(|slot| *slot.borrow_mut() = std::mem::take(&mut self.0));
    }
}
/// Hold only weak Control identity across the main loop and exit drain; store no Application pointer or grant query callbacks.
pub(super) fn with_owner<T>(control: &Rc<Control>, operation: impl FnOnce() -> T) -> T {
    let _scope = OwnerScope(OWNER.with(|slot| slot.replace(Rc::downgrade(control))));
    operation()
}
fn active() -> Option<Rc<Control>> {
    OWNER.with(|slot| slot.borrow().upgrade())
}
/// Reuse existing control subclasses to intercept synchronous edits and ordinary input; destruction/cancellation/reads retain original cleanup.
/// WM_GESTURE/WM_TOUCH carry system handles that host/control default procedures must release; filters without lp must not swallow them.
pub(super) fn blocks_input(message: u32) -> bool {
    active().is_some_and(|owner| owner.paused())
        && ((WM_KEYFIRST..=WM_KEYLAST).contains(&message)
            || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&message)
            || matches!(
                message,
                WM_SETTEXT
                    | WM_PASTE
                    | WM_CUT
                    | WM_CLEAR
                    | WM_UNDO
                    | WM_IME_STARTCOMPOSITION
                    | WM_IME_COMPOSITION
                    | WM_IME_ENDCOMPOSITION
                    | WM_COMMAND
                    | WM_SYSCOMMAND
                    | WM_CLOSE
                    | CB_SETCURSEL
                    | CB_SHOWDROPDOWN
            )
            || message == windows::Win32::UI::Controls::EM_REPLACESEL)
}
struct Scope(Option<Query>);
impl Drop for Scope {
    fn drop(&mut self) {
        QUERY.with(|slot| slot.set(self.0));
    }
}

/// callback only borrows the application; wait must not modify it. The pointer stays within this thread/stack scope and is revoked on unwind too.
/// Do not install callbacks on Application's mutable processing stack, avoiding aliased &mut Application borrows through synchronous window messages.
pub(super) fn with_query<T>(
    control: &Rc<Control>,
    callback: impl Fn() -> bool,
    wait: impl FnOnce() -> T,
) -> T {
    unsafe fn invoke<F: Fn() -> bool>(pointer: *const ()) -> bool {
        (&*(pointer as *const F))()
    }
    fn query<F: Fn() -> bool>(control: &Rc<Control>, callback: &F) -> Query {
        Query {
            owner: Rc::as_ptr(control),
            context: callback as *const F as *const (),
            call: invoke::<F>,
        }
    }
    let _scope = Scope(QUERY.with(|slot| slot.replace(Some(query(control, &callback)))));
    wait()
}
fn ready(control: &Rc<Control>) -> bool {
    QUERY.with(|slot| {
        let query = slot.take();
        let _scope = Scope(query); // If read-only Win32 calls in the query reenter, do not recursively reuse the active callback.
        query.is_some_and(|query| {
            query.owner == Rc::as_ptr(control) && unsafe { (query.call)(query.context) }
        })
    })
}

pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &super::State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    dispatch_control(
        hwnd,
        &state.session_end,
        &state.session_end_reason,
        msg,
        wp,
        lp,
    )
}
pub(super) unsafe fn dispatch_active(
    hwnd: HWND,
    reason: &Cell<bool>,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    let control = active()?;
    dispatch_control(hwnd, &control, reason, msg, wp, lp)
}
unsafe fn dispatch_control(
    hwnd: HWND,
    control: &Rc<Control>,
    reason: &Cell<bool>,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    let clear_reason = || {
        if reason.replace(false) {
            if let Err(error) = ShutdownBlockReasonDestroy(hwnd) {
                eprintln!("native-session-end-reason-cleanup: {error}");
            }
        }
    };
    match msg {
        WM_QUERYENDSESSION => {
            let accepted = control.paused() || ready(control);
            if accepted {
                clear_reason();
                if !control.ended() {
                    control.0.set(Phase::Queried);
                }
            } else {
                if !reason.get()
                    && ShutdownBlockReasonCreate(
                        hwnd,
                        &windows::core::HSTRING::from(crate::ui_text::text(
                            "action-session-unsaved",
                        )),
                    )
                    .is_ok()
                {
                    reason.set(true);
                }
                // After returning FALSE, advance existing checkpoints through the original message pump; no saving, dialogs, or waiting inside synchronous system callbacks.
                let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            }
            Some(LRESULT(isize::from(accepted)))
        }
        WM_ENDSESSION => {
            clear_reason();
            if wp.0 != 0 {
                if !control.ended() {
                    control.0.set(Phase::Ended);
                    PostQuitMessage(0);
                }
            } else if !control.ended() {
                control.0.set(Phase::Running);
                let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            }
            Some(LRESULT(0))
        }
        WM_DESTROY | WM_NCDESTROY => {
            clear_reason();
            None
        }
        _ if control.paused() => {
            if msg == WM_DROPFILES {
                windows::Win32::UI::Shell::DragFinish(windows::Win32::UI::Shell::HDROP(
                    wp.0 as *mut _,
                ));
            }
            // DefWindowProc's WM_CLOSE/SC_CLOSE destroys documents; do not delete Units after the approved point.
            Some(if matches!(msg, WM_CLOSE | WM_SYSCOMMAND | WM_DROPFILES) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            })
        }
        _ => None,
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_end_scope_tests.rs"]
mod tests;
