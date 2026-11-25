//! 菜单层级的固定标量请求；独立请求 HWND 不复用 B 的输入槽，属性本身不是鉴权。
//! 取消先撤 COMMIT 再看 CLAIMED；已领取请求须保留至 FINISHED，不能提前覆盖或退休。
use crate::{native_popup::Lease, native_protocol::value};
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

pub const MESSAGE_NAME: PCWSTR = w!("Otter.Live.Popup.Layer.Request.v1");
pub const TOKEN: PCWSTR = w!("Otter.Live.Popup.Layer.Token.v1");
pub const ID: PCWSTR = w!("Otter.Live.Popup.Layer.Id.v1");
pub const COMMIT: PCWSTR = w!("Otter.Live.Popup.Layer.Commit.v1");
pub const OPERATION: PCWSTR = w!("Otter.Live.Popup.Layer.Operation.v1");
pub const POPUP: PCWSTR = w!("Otter.Live.Popup.Layer.Popup.v1");
pub const OWNER: PCWSTR = w!("Otter.Live.Popup.Layer.Owner.v1");
pub const COOKIE: PCWSTR = w!("Otter.Live.Popup.Layer.Cookie.v1");
pub const EPOCH: PCWSTR = w!("Otter.Live.Popup.Layer.Epoch.v1");
pub const ROOT: PCWSTR = w!("Otter.Live.Popup.Layer.Root.v1");
pub const INPUT: PCWSTR = w!("Otter.Live.Popup.Layer.Input.v1");
pub const INSTANCE: PCWSTR = w!("Otter.Live.Popup.Layer.Instance.v1");
pub const CLAIMED: PCWSTR = w!("Otter.Live.Popup.Layer.Claimed.v1");
pub const RESULT: PCWSTR = w!("Otter.Live.Popup.Layer.Result.v1");
pub const FINISHED: PCWSTR = w!("Otter.Live.Popup.Layer.Finished.v1");
pub const PENDING: windows::core::HRESULT = windows::core::HRESULT(0x8000000Au32 as i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Operation {
    Acquire = 1,
    Restore = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub window: HWND,
    pub token: usize,
    pub nonce: usize,
    pub operation: Operation,
    pub lease: Lease,
}
impl Request {
    fn snapshot(window: HWND) -> Option<Self> {
        Some(Self {
            window,
            token: value(window, TOKEN),
            nonce: value(window, ID),
            operation: match value(window, OPERATION) {
                1 => Operation::Acquire,
                2 => Operation::Restore,
                _ => return None,
            },
            lease: Lease {
                popup: HWND(value(window, POPUP) as *mut _),
                owner: HWND(value(window, OWNER) as *mut _),
                cookie: value(window, COOKIE),
                epoch: value(window, EPOCH),
                root: HWND(value(window, ROOT) as *mut _),
                request: value(window, INPUT),
                instance: value(window, INSTANCE),
            },
        })
    }
    /// 只比较不可变请求身份；取消 COMMIT 后仍可给原已领取请求写完成结果。
    pub fn matches(self) -> bool {
        self.token != 0 && self.nonce != 0 && Self::snapshot(self.window) == Some(self)
    }
    /// 源线程入口，不能把同 PID、同线程或恢复记录本身当作新输入权限。
    pub fn read(window: HWND, nonce: usize, receiving: HWND) -> Option<Self> {
        let request = Self::snapshot(window)?;
        let lease = request.lease;
        if nonce == 0
            || request.nonce != nonce
            || request.token == 0
            || value(window, COMMIT) != nonce
            || lease.popup != receiving
            || lease.popup == lease.root
            || lease.instance == 0
            || lease.request == 0
            || lease.cookie == 0
            || lease.epoch == 0
            || lease.owner.0.is_null()
            || lease.root.0.is_null()
            || [lease.popup, lease.owner, lease.root].contains(&window)
        {
            return None;
        }
        unsafe {
            let mut process = 0;
            let thread = GetWindowThreadProcessId(receiving, Some(&mut process));
            if thread != GetCurrentThreadId()
                || process != GetCurrentProcessId()
                || GetAncestor(receiving, GA_ROOT) != receiving
                || GetWindowThreadProcessId(window, None) == 0
            {
                return None;
            }
        }
        (request.matches() && value(window, COMMIT) == nonce).then_some(request)
    }
    pub fn active(self) -> bool {
        Self::read(self.window, self.nonce, self.lease.popup) == Some(self)
    }
    /// CLAIMED 在最后一次 COMMIT 复读之前发布；未通过复读绝不修改源窗口。
    pub fn claim(self) -> windows::core::Result<bool> {
        if !self.active() || value(self.window, CLAIMED) == self.nonce {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.window, CLAIMED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        if !self.active() {
            self.finish(E_ABORT)?;
            return Ok(false);
        }
        Ok(true)
    }
    pub fn result(self) -> Option<windows::core::HRESULT> {
        // 普通轮询先查提交标记；只有候选回执才需要重读完整请求身份。
        (value(self.window, FINISHED) == self.nonce
            && value(self.window, CLAIMED) == self.nonce
            && self.matches())
        .then(|| windows::core::HRESULT(value(self.window, RESULT) as u32 as i32))
    }
    /// RESULT 先写、FINISHED 最后提交；源调用返回后才能调用，不能将取消当完成。
    pub fn finish(self, result: windows::core::HRESULT) -> windows::core::Result<()> {
        if self.matches() && value(self.window, CLAIMED) == self.nonce {
            unsafe {
                SetPropW(
                    self.window,
                    RESULT,
                    Some(HANDLE(result.0 as u32 as usize as *mut _)),
                )?;
                SetPropW(self.window, FINISHED, Some(HANDLE(self.nonce as *mut _)))?;
                // 只唤醒原请求窗口；通知失败不撤回已发布结果，宿主仍须保留截止唤醒。
                if let Err(error) = PostMessageW(Some(self.window), WM_NULL, WPARAM(0), LPARAM(0)) {
                    eprintln!("native-live-popup-layer-notify: {error}");
                }
            }
        }
        Ok(())
    }
}
