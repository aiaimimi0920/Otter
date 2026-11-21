//! 本地原生按钮/悬浮事务的标量协议；属性不是鉴权，宿主仍执行权限与 Destination 预检。
//! 独立 epoch 区分暂停/恢复，nonce 区分按钮事务；不复用随鼠标 Leave 撤销的 hover ACTIVE。
use crate::scope;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*},
};

pub const MESSAGE_NAME: PCWSTR = w!("Otter.Live.Native.Input.v3");
pub const EPOCH: PCWSTR = w!("Otter.Live.Native.Epoch.v1");
pub const COMMIT: PCWSTR = w!("Otter.Live.Native.Commit.v1");
pub const COOKIE: PCWSTR = w!("Otter.Live.Native.Cookie.v1");
pub const REQUEST_EPOCH: PCWSTR = w!("Otter.Live.Native.RequestEpoch.v1");
pub const ROOT: PCWSTR = w!("Otter.Live.Native.Root.v1");
pub const DESTINATION: PCWSTR = w!("Otter.Live.Native.Destination.v1");
pub const MESSAGE: PCWSTR = w!("Otter.Live.Native.Message.v1");
pub const WP: PCWSTR = w!("Otter.Live.Native.WParam.v1");
pub const LP: PCWSTR = w!("Otter.Live.Native.LParam.v1");
pub const CLAIMED: PCWSTR = w!("Otter.Live.Native.Claimed.v1");
pub const FINISHED: PCWSTR = w!("Otter.Live.Native.Finished.v1");
pub const REQUEST_KEYS: [PCWSTR; 10] = [
    COMMIT,
    COOKIE,
    REQUEST_EPOCH,
    ROOT,
    DESTINATION,
    MESSAGE,
    WP,
    LP,
    CLAIMED,
    FINISHED,
];

pub fn request_keys() -> impl Iterator<Item = PCWSTR> {
    REQUEST_KEYS
        .into_iter()
        .chain(crate::native_input_scope::keys())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub owner: HWND,
    pub cookie: usize,
    pub epoch: usize,
    pub root: HWND,
    pub destination: HWND,
    pub nonce: usize,
    pub message: u32,
    pub wp: usize,
    pub lp: isize,
    pub popup_scope: crate::native_input_scope::Snapshot,
}

pub fn value(hwnd: HWND, key: PCWSTR) -> usize {
    unsafe { GetPropW(hwnd, key).0 as usize }
}

/// 固定按钮与同步 motion/hover 白名单；不接受任意 Win32 消息或跨进程指针。
pub fn valid_packet(message: u32, wp: usize, lp: isize) -> bool {
    matches!(
        message,
        WM_LBUTTONDOWN
            | WM_LBUTTONDBLCLK
            | WM_LBUTTONUP
            | WM_RBUTTONUP
            | WM_MOUSEMOVE
            | windows::Win32::UI::Controls::WM_MOUSEHOVER
    ) && wp & !0x7f == 0
        && u32::try_from(lp).is_ok()
}

impl Request {
    fn snapshot(owner: HWND, nonce: usize) -> Option<Self> {
        if nonce == 0 || value(owner, COMMIT) != nonce {
            return None;
        }
        Some(Self {
            owner,
            cookie: value(owner, COOKIE),
            epoch: value(owner, REQUEST_EPOCH),
            root: HWND(value(owner, ROOT) as *mut _),
            destination: HWND(value(owner, DESTINATION) as *mut _),
            nonce,
            message: u32::try_from(value(owner, MESSAGE)).ok()?,
            wp: value(owner, WP),
            lp: value(owner, LP) as isize,
            popup_scope: crate::native_input_scope::Snapshot::read(owner)?,
        })
    }

    /// 在源消息线程读取；首尾重读提交状态，拒绝半发布、撤销和跨线程目标。
    pub fn read(owner: HWND, nonce: usize, receiving: HWND) -> Option<Self> {
        let request = Self::snapshot(owner, nonce)?;
        if request.destination != receiving
            || request.cookie == 0
            || request.epoch == 0
            || !valid_packet(request.message, request.wp, request.lp)
        {
            return None;
        }
        unsafe {
            if !IsWindowVisible(owner).as_bool()
                || IsIconic(owner).as_bool()
                || value(owner, scope::COOKIE) != request.cookie
                || value(owner, EPOCH) != request.epoch
                || GetAncestor(request.root, GA_ROOT) != request.root
                || GetAncestor(receiving, GA_ROOT) != request.root
                || IsIconic(request.root).as_bool()
            {
                return None;
            }
            let mut root_process = 0;
            let mut destination_process = 0;
            let root_thread = GetWindowThreadProcessId(request.root, Some(&mut root_process));
            let destination_thread =
                GetWindowThreadProcessId(receiving, Some(&mut destination_process));
            if root_process != GetCurrentProcessId()
                || destination_process != root_process
                || root_thread != GetCurrentThreadId()
                || destination_thread != root_thread
            {
                return None;
            }
        }
        (request.popup_scope.valid_for(request.root, owner)
            && Self::snapshot(owner, nonce) == Some(request)
            && value(owner, scope::COOKIE) == request.cookie
            && value(owner, EPOCH) == request.epoch)
            .then_some(request)
    }

    /// 同一源线程上的多个 hook 必须先领取再调用应用，防止重复执行一笔按钮。
    /// 调用方仍须校验嵌套来源并保存原 TLS scope，不能覆盖正在运行的事务。
    pub fn claim(self) -> windows::core::Result<bool> {
        if Self::read(self.owner, self.nonce, self.destination) != Some(self)
            || value(self.owner, CLAIMED) == self.nonce
        {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.owner, CLAIMED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        Ok(Self::read(self.owner, self.nonce, self.destination) == Some(self))
    }

    fn same_publication(self) -> bool {
        Self::snapshot(self.owner, self.nonce) == Some(self)
            && value(self.owner, scope::COOKIE) == self.cookie
            && value(self.owner, CLAIMED) == self.nonce
            && value(self.owner, COMMIT) == self.nonce
    }

    /// 只由已领取并执行完窗口过程的 dispatch 调用，不是一次新的输入授权。
    /// 点击可合法销毁控件或撤销控制；只确认原 publication，不能据当前源状态抹掉执行结果。
    pub(crate) fn finish_returned(self) -> windows::core::Result<bool> {
        if !self.same_publication() {
            return Ok(false);
        }
        unsafe {
            SetPropW(self.owner, FINISHED, Some(HANDLE(self.nonce as *mut _)))?;
        }
        Ok(self.same_publication() && value(self.owner, FINISHED) == self.nonce)
    }
}

#[cfg(test)]
#[path = "native_protocol_tests.rs"]
mod tests;
