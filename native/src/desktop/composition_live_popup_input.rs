//! 附属镜像输入许可与组内焦点移交；不改变普通 Live 的暂停/释放语义。
use super::*;
use crate::live_hover_owner::Control as ParentControl;
use crate::live_window_target::{popup::Scope, Target};
#[cfg(test)]
#[path = "composition_live_popup_focus_tests.rs"]
mod popup_focus_tests;

#[derive(Default)]
pub(in crate::desktop) struct Focus {
    armed: bool,
    pending: Option<HWND>,
    active: Option<Scope>,
    family: Vec<Scope>,
    // 只给失败恢复去重，不构成输入授权；暂停/失焦不能清掉这条历史而触发同周期重试。
    parent_cycle: Option<(usize, usize, usize)>,
}
impl Focus {
    pub(super) fn clear(&mut self) {
        self.pending = None;
        self.active = None;
    }
}
impl LiveWindow {
    pub(in crate::desktop) fn popup_scopes(&self, scopes: Vec<Scope>) {
        // 仅由本 Session 的有界附属集合提供；仍在实际焦点转移时逐项重验。
        self.state.popup_focus.borrow_mut().family = scopes;
    }
    pub(in crate::desktop) fn enable_popup_focus(&self) {
        self.state.popup_focus.borrow_mut().armed = true;
    }
    pub(in crate::desktop) fn focused_source_popup(&self) -> Option<Scope> {
        // 只暴露 Session 已核验的 active，WndProc 暂存的 pending 不是发现凭据。
        self.popup_control_allowed()
            .then(|| self.state.popup_focus.borrow().active.clone())
            .flatten()
    }
    pub(in crate::desktop) fn resolve_popup_focus(&self, root: Option<&Target>) {
        let checking = {
            let focus = self.state.popup_focus.borrow();
            focus.pending.is_some() || focus.active.is_some()
        };
        if checking {
            self.resolve_popup_focus_at(root, unsafe { GetForegroundWindow() });
        }
    }
    // 只在 Session poll 中验证源身份，不在 WndProc 中打开进程或遍历 owner 链。
    fn resolve_popup_focus_at(&self, root: Option<&Target>, foreground: HWND) {
        let (pending, active, family) = {
            let mut focus = self.state.popup_focus.borrow_mut();
            (
                focus.pending.take(),
                focus.active.take(),
                focus.family.clone(),
            )
        };
        if pending.is_none() && active.is_none() {
            return;
        }
        if !self.popup_control_allowed() {
            return; // 停止/远程接管之后不得因迟到的焦点判断重新启用。
        }
        let foreground = unsafe { GetAncestor(foreground, GA_ROOT) };
        if foreground == self.hwnd || self.state.popup_peers.borrow().contains(&foreground) {
            return;
        }
        let scope = root.and_then(|root| {
            if let Some(next) = pending {
                if unsafe { GetAncestor(next, GA_ROOT) } != foreground {
                    return None;
                }
            }
            match active {
                Some(scope) if scope.target().is_ok_and(|t| t.hwnd() == foreground) => Some(scope),
                _ => Scope::bind(root, self.hover_identity(), foreground)
                    .ok()
                    .or_else(|| {
                        family.into_iter().find(|scope| {
                            scope
                                .target()
                                .is_ok_and(|target| target.hwnd() == foreground)
                        })
                    }),
            }
        });
        if let Some(scope) = scope.filter(|_| unsafe {
            IsWindowVisible(foreground).as_bool() && !IsIconic(foreground).as_bool()
        }) {
            self.state.popup_focus.borrow_mut().active = Some(scope);
        } else {
            unsafe {
                suspend(self.hwnd, &self.state);
                cancel_peers(self.hwnd, &self.state);
            }
        }
    }
    pub(in crate::desktop) fn popup_control_allowed(&self) -> bool {
        let input = self.state.input.borrow();
        input.allowed
            && input.desired
            && !input.remote
            && !input.failed
            && (input.enabled || input.pending)
            && !self.state.group_locked.get()
    }
    pub(in crate::desktop) fn popup_control_grant(&self) -> Option<ParentControl> {
        self.popup_control_allowed()
            .then(|| self.hover_identity()?.capture_control().ok())
            .flatten()
    }
    pub(in crate::desktop) fn sync_popup_control(&self, parent: Option<ParentControl>) {
        let parent = parent.filter(|control| control.valid());
        let renewed = parent.is_some_and(|control| {
            let owner = control.identity();
            let cycle = (owner.hwnd, owner.cookie, control.epoch());
            self.state
                .popup_focus
                .borrow_mut()
                .parent_cycle
                .replace(cycle)
                != Some(cycle)
        });
        let allow = parent.is_some();
        let input = self.state.input.borrow();
        if input.desired == allow || (allow && input.failed && !renewed) {
            return;
        }
        drop(input);
        if allow {
            self.state.input.borrow_mut().allowed = true;
            self.request_default_input();
        } else {
            unsafe {
                cancel(self.hwnd, &self.state);
            }
        }
    }
}
pub(super) unsafe fn focus_handoff(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    let next = match message {
        WM_KILLFOCUS => HWND(wp.0 as *mut _),
        WM_ACTIVATE => HWND(lp.0 as *mut _),
        WM_CAPTURECHANGED => GetForegroundWindow(),
        _ => HWND::default(),
    };
    focus_handoff_to(hwnd, state, message, next)
}
unsafe fn focus_handoff_to(hwnd: HWND, state: &State, message: u32, next: HWND) -> bool {
    if message == WM_CAPTURECHANGED && (next == hwnd || state.popup_peers.borrow().contains(&next))
    {
        return false; // 无非镜像前台候选，仍按捕获丢失取消；不凭 capture HWND 授权源。
    }
    if !next.0.is_null() && state.popup_peers.borrow().contains(&next) {
        state.popup_focus.borrow_mut().pending = None;
        return state.input.borrow_mut().handoff_keys();
    }
    // 源在 down 内激活 dialog 时，不能用停止 worker 去释放按住状态，否则也会取消
    // 仍在途的源模态调用。先按原 FIFO 排 up，Session 在后续输入处理前核验唯一候选。
    let defer = !next.0.is_null()
        && resumable(state)
        && !state.menu_open.get()
        && state.popup_focus.borrow().armed
        && {
            let mut input = state.input.borrow_mut();
            (input.enabled || input.pending) && input.handoff_source()
        };
    if defer {
        state.popup_focus.borrow_mut().pending = Some(next);
        // 不跨 ReleaseCapture 持有借用；重入 WM_CAPTURECHANGED 已没有 UI held 按钮。
        if GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
        return true;
    }
    state.popup_focus.borrow_mut().clear();
    if state.popup.get() && message == WM_KILLFOCUS {
        // 焦点离开整组时撤销父 B；组内移交不能触发源模态菜单取消。
        cancel_peers(hwnd, state);
    }
    false
}

unsafe fn cancel_peers(hwnd: HWND, state: &State) {
    for peer in state
        .popup_peers
        .borrow()
        .iter()
        .filter(|peer| **peer != hwnd)
    {
        if let Err(error) = PostMessageW(Some(*peer), WM_CANCELMODE, WPARAM(0), LPARAM(0)) {
            eprintln!("native-live-popup-focus-cancel: {error}");
        }
    }
}
