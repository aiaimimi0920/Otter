//! B 的悬浮资格由窗口输入层拥有；暂停/离开同步撤销，不等待输入 worker 排空。
use otter_live_hover_hook::native_protocol;
use otter_live_hover_hook::scope::*;
use std::cell::Cell;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*, Security::Cryptography::*, System::Threading::GetCurrentProcessId,
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Clone, Copy)]
pub(crate) struct Identity {
    pub hwnd: usize,
    pub cookie: usize,
}
impl Identity {
    /// 在 UI 准入时固定控制周期；worker 不得在发布旧操作时重新领取当前 epoch。
    pub(crate) fn capture_control(self) -> Result<Control> {
        let epoch = native_protocol::value(self.window(), native_protocol::EPOCH);
        let control = Control {
            identity: self,
            epoch,
        };
        if !control.valid() {
            return Err(Error::new(E_ABORT, "Live control cycle is not active"));
        }
        Ok(control)
    }
    pub fn window(self) -> HWND {
        HWND(self.hwnd as *mut _)
    }
    pub fn token(self) -> HANDLE {
        HANDLE(self.cookie as *mut _)
    }
    pub fn valid(self) -> bool {
        unsafe {
            let mut process = 0;
            GetWindowThreadProcessId(self.window(), Some(&mut process));
            process == GetCurrentProcessId()
                && self.cookie != 0
                && GetPropW(self.window(), COOKIE) == self.token()
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Control {
    identity: Identity,
    epoch: usize,
}
impl Control {
    pub(crate) fn identity(self) -> Identity {
        self.identity
    }
    pub(crate) fn epoch(self) -> usize {
        self.epoch
    }
    pub(crate) fn valid(self) -> bool {
        self.epoch != 0
            && self.identity.valid()
            && native_protocol::value(self.identity.window(), native_protocol::EPOCH) == self.epoch
    }
}
pub(crate) struct Owner(Identity, Cell<usize>);
impl Owner {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let mut bytes = [0; std::mem::size_of::<usize>()];
        unsafe {
            BCryptGenRandom(None, &mut bytes, BCRYPT_USE_SYSTEM_PREFERRED_RNG).ok()?;
        }
        let cookie = usize::from_ne_bytes(bytes);
        if cookie == 0 || cookie == usize::MAX {
            return Err(Error::new(E_FAIL, "Invalid Live hover nonce"));
        }
        let identity = Identity {
            hwnd: hwnd.0 as usize,
            cookie,
        };
        unsafe {
            SetPropW(hwnd, COOKIE, Some(identity.token()))?;
        }
        let owner = Self(identity, Cell::new(0));
        owner.control(false)?;
        Ok(owner)
    }
    pub fn identity(&self) -> Identity {
        self.0
    }
    /// 独立于鼠标 Leave：只有控制暂停/恢复才更新弹窗租约的代数。
    pub fn control(&self, enabled: bool) -> Result<()> {
        if !self.0.valid() {
            return Ok(());
        }
        let current = native_protocol::value(self.0.window(), native_protocol::EPOCH);
        if enabled && current != 0 {
            return Ok(());
        }
        let epoch = if enabled {
            let next = self
                .1
                .get()
                .checked_add(1)
                .ok_or_else(|| Error::new(E_FAIL, "Live control epoch exhausted"))?;
            self.1.set(next);
            next
        } else {
            0
        };
        unsafe {
            SetPropW(
                self.0.window(),
                native_protocol::EPOCH,
                Some(HANDLE(epoch as *mut _)),
            )
        }
    }
    pub fn active(&self, active: bool) -> Result<()> {
        if self.0.valid() {
            unsafe {
                SetPropW(
                    self.0.window(),
                    ACTIVE,
                    Some(if active {
                        self.0.token()
                    } else {
                        HANDLE::default()
                    }),
                )?;
            }
        }
        Ok(())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.0.valid() {
            unsafe {
                // 属性随 HWND 一起消失；复用后的 HWND 不得被旧 owner 写入。
                for key in [ACTIVE, SOURCE, COOKIE, native_protocol::EPOCH] {
                    let _ = RemovePropW(self.0.window(), key);
                }
                for key in native_protocol::request_keys() {
                    let _ = RemovePropW(self.0.window(), key);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "live_hover_owner_tests.rs"]
mod tests;
