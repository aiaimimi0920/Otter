//! 独立恢复 owner 的单槽 posted 请求；未知结果保留原窗口/身份，不阻塞 UI、不重放输入。
use crate::{
    live_native_hooks::{Hooks, Kind},
    live_window_target::Target,
};
use otter_live_hover_hook::{
    native_layer_dispatch, native_layer_protocol as p, native_popup::Lease,
    native_popup_layer::MANAGED, native_protocol::value,
};
use std::time::{Duration, Instant};
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, Security::Cryptography::*, UI::WindowsAndMessaging::*},
};

const BUDGET: Duration = Duration::from_millis(250);
pub const POLL: Duration = Duration::from_millis(16);
fn in_flight() -> Error {
    Error::new(p::PENDING, "Popup layer request is pending")
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        if let Err(error) = unsafe { DestroyWindow(self.0) } {
            eprintln!("native-live-popup-request-window: {error}");
        }
    }
}
#[derive(Clone, Copy)]
struct Pending {
    request: p::Request,
    deadline: Instant,
    timed_out: bool,
}
pub struct Channel {
    target: Target,
    lease: Lease,
    window: Window,
    _hooks: Hooks,
    token: usize,
    next: usize,
    pending: Option<Pending>,
    retry_at: Option<Instant>,
    acquired: bool,
}
impl Channel {
    pub fn notification_window(&self) -> HWND {
        self.window.0
    }
    pub fn new(target: &Target, lease: Lease) -> Result<Self> {
        target.validate()?;
        let hooks = Hooks::install(target, Kind::Layer)?;
        let window = Window(unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )?
        });
        let mut bytes = [0; std::mem::size_of::<usize>()];
        unsafe {
            BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG).ok()?;
        }
        let token = usize::from_ne_bytes(bytes);
        if token == 0 || token == usize::MAX {
            return Err(Error::new(E_FAIL, "Invalid popup request identity"));
        }
        target.validate()?;
        if !lease.same_instance() {
            return Err(Error::new(E_ABORT, "Popup display instance changed"));
        }
        Ok(Self {
            target: target.clone(),
            lease,
            window,
            _hooks: hooks,
            token,
            next: 1,
            pending: None,
            retry_at: None,
            acquired: false,
        })
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.pending
            .map(|p| {
                let poll = Instant::now() + POLL;
                if p.timed_out {
                    poll
                } else {
                    poll.min(p.deadline)
                }
            })
            .into_iter()
            .chain(self.retry_at)
            .min()
    }
    pub fn acquire(&mut self) -> Result<()> {
        self.target.validate()?;
        if !self.lease.valid() {
            return Err(Error::new(E_ABORT, "Popup display authorization changed"));
        }
        if self.acquired
            && value(self.lease.popup, MANAGED) == self.lease.instance
            && unsafe { GetWindowLongPtrW(self.lease.popup, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0
                != 0
        {
            return Ok(());
        }
        self.acquired = false;
        self.run(p::Operation::Acquire)
    }
    pub fn restore(&mut self) -> Result<()> {
        // 原显示对象已消失是权威终态；迟到 Acquire 也因同一实例检查不能再修改新对象。
        if !self.lease.same_instance() {
            self.withdraw()?;
            self.pending = None;
            self.retry_at = None;
            self.acquired = false;
            return Ok(());
        }
        self.cancel_acquire()?;
        self.target.validate()?;
        self.run(p::Operation::Restore)
    }
    fn withdraw(&self) -> Result<()> {
        if let Some(pending) = self.pending {
            if pending.request.matches() {
                unsafe {
                    SetPropW(self.window.0, p::COMMIT, Some(HANDLE::default()))?;
                }
            }
        }
        Ok(())
    }
    fn cancel_acquire(&mut self) -> Result<()> {
        let Some(pending) = self
            .pending
            .filter(|p| p.request.operation == p::Operation::Acquire)
        else {
            return Ok(());
        };
        // 顺序不可颠倒：先撤 COMMIT，未领取的旧快照在 CLAIMED 后复读时必被拒绝。
        self.withdraw()?;
        if !pending.request.matches() {
            return Err(Error::new(E_ABORT, "Popup request identity changed"));
        }
        if pending.request.result().is_some()
            || value(self.window.0, p::CLAIMED) != pending.request.nonce
        {
            self.pending = None;
            self.acquired = false;
            return Ok(());
        }
        // 已领取但未完成时保留原槽；不能先发布 Restore 或先退休 journal。
        let _ = self.observe()?;
        self.acquired = false;
        Ok(())
    }
    fn run(&mut self, operation: p::Operation) -> Result<()> {
        if let Some(pending) = self.pending {
            if pending.request.operation != operation {
                return Err(Error::new(
                    E_ABORT,
                    "Popup layer operation is still in flight",
                ));
            }
            let result = self.observe()?;
            if result == p::PENDING {
                // 源端明确在修改前拒绝忙状态，才允许稍后新请求；未知结果绝不走此分支。
                self.retry_at = Some(Instant::now() + POLL);
                return Err(in_flight());
            }
            result.ok()?;
            self.acquired = operation == p::Operation::Acquire;
            self.retry_at = None;
            return Ok(());
        }
        if self.retry_at.is_some_and(|at| Instant::now() < at) {
            return Err(in_flight());
        }
        self.retry_at = None;
        self.publish(operation)?;
        Err(in_flight())
    }
    fn observe(&mut self) -> Result<windows::core::HRESULT> {
        let Some(mut pending) = self.pending else {
            return Err(Error::new(E_UNEXPECTED, "Missing popup layer request"));
        };
        if let Some(result) = pending.request.result() {
            self.pending = None;
            return Ok(result);
        }
        if !pending.request.matches() {
            return Err(Error::new(E_ABORT, "Popup request identity changed"));
        }
        self.target.validate()?;
        if !pending.timed_out && Instant::now() >= pending.deadline {
            self.withdraw()?;
            pending.timed_out = true;
            self.pending =
                (value(self.window.0, p::CLAIMED) == pending.request.nonce).then_some(pending);
            return Err(Error::new(
                ERROR_TIMEOUT.to_hresult(),
                "Popup layer source did not complete",
            ));
        }
        Err(in_flight())
    }
    fn publish(&mut self, operation: p::Operation) -> Result<()> {
        self.target.validate()?;
        let nonce = self.next;
        self.next = nonce
            .checked_add(1)
            .ok_or_else(|| Error::new(E_FAIL, "Popup request id exhausted"))?;
        let id = native_layer_dispatch::message_id();
        if id == 0 {
            return Err(Error::from_win32());
        }
        let request = p::Request {
            window: self.window.0,
            token: self.token,
            nonce,
            operation,
            lease: self.lease,
        };
        unsafe {
            for (key, number) in [
                (p::COMMIT, 0),
                (p::TOKEN, self.token),
                (p::ID, nonce),
                (p::OPERATION, operation as usize),
                (p::POPUP, self.lease.popup.0 as usize),
                (p::OWNER, self.lease.owner.0 as usize),
                (p::COOKIE, self.lease.cookie),
                (p::EPOCH, self.lease.epoch),
                (p::ROOT, self.lease.root.0 as usize),
                (p::INPUT, self.lease.request),
                (p::INSTANCE, self.lease.instance),
                (p::CLAIMED, 0),
                (p::RESULT, 0),
                (p::FINISHED, 0),
                (p::COMMIT, nonce),
            ] {
                SetPropW(self.window.0, key, Some(HANDLE(number as *mut _)))?;
            }
            self.pending = Some(Pending {
                request,
                deadline: Instant::now() + BUDGET,
                timed_out: false,
            });
            if let Err(error) = PostMessageW(
                Some(self.target.hwnd()),
                id,
                WPARAM(self.window.0 .0 as usize),
                LPARAM(nonce as isize),
            ) {
                // PostMessage 失败没有入队；仍先撤提交，再确认未领取才清槽。
                self.withdraw()?;
                if value(self.window.0, p::CLAIMED) != nonce {
                    self.pending = None;
                }
                return Err(error);
            }
        }
        Ok(())
    }
}
impl Drop for Channel {
    fn drop(&mut self) {
        // 正常 owner 必须先 drain 恢复；异常析构撤销迟到 Acquire，独立 child 保留记录。
        if let Err(error) = self.withdraw() {
            eprintln!("native-live-popup-request-cancel: {error}");
        }
    }
}

#[cfg(test)]
#[path = "live_popup_layer_request_tests.rs"]
mod tests;
