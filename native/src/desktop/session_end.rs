//! 系统结束会话准入；只在 Application 的只读等待作用域查询，不把普通关窗当作系统退出。
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use windows::{
    core::w,
    Win32::{Foundation::*, System::Shutdown::*, UI::WindowsAndMessaging::*},
};

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
/// 仅持弱Control身份，覆盖主循环及退出排空；不储存Application指针或授予查询回调。
pub(super) fn with_owner<T>(control: &Rc<Control>, operation: impl FnOnce() -> T) -> T {
    let _scope = OwnerScope(OWNER.with(|slot| slot.replace(Rc::downgrade(control))));
    operation()
}
fn active() -> Option<Rc<Control>> {
    OWNER.with(|slot| slot.borrow().upgrade())
}
/// 复用现有控件subclass，拦截同步编辑以及普通输入；销毁/取消/读取仍走原清理过程。
/// WM_GESTURE/WM_TOUCH携带系统句柄，必须交宿主/控件默认过程释放，不能在无lp的过滤器吞掉。
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

/// callback 仅借用应用；wait 不得修改该应用。指针不离开当前线程/栈作用域，unwind 也撤回。
/// 不在 Application 可变处理栈安装回调，避免同步窗口消息别名借用 &mut Application。
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
        let _scope = Scope(query); // 查询里的Win32只读调用若重入，不递归沿用正在执行的回调。
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
                        w!("工作区尚未安全保存，请完成操作或先退出 Otter。"),
                    )
                    .is_ok()
                {
                    reason.set(true);
                }
                // 返回FALSE后沿原消息泵推进既有检查点，不在系统同步回调中保存、弹框或等待。
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
            // DefWindowProc 的 WM_CLOSE/SC_CLOSE 会销毁文档，不能在已批准的时间点之后删除Unit。
            Some(if matches!(msg, WM_CLOSE | WM_SYSCOMMAND | WM_DROPFILES) {
                LRESULT(0)
            } else {
                DefWindowProcW(hwnd, msg, wp, lp)
            })
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "session_end_scope_tests.rs"]
mod tests;
