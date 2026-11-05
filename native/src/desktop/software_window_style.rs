//! 自有 HWND 的软件呈现样式事务；调用期间不得持有会被窗口过程重入借用的状态。
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};

const MASK: u32 = WS_EX_LAYERED.0;

/// 在 UI 线程完成创建/首次提交后 commit；失败或 unwind 时只恢复本事务拥有的样式位。
/// 不负责解绑 DirectComposition，也不释放 GPU 对象；不能用于仍有关联合成树的窗口。
pub struct Transition<'a> {
    hwnd: HWND,
    original: u32,
    desired: u32,
    active: bool,
    owns: Option<&'a dyn Fn() -> bool>,
    _thread: PhantomData<Rc<()>>,
}

impl<'a> Transition<'a> {
    /// # Safety
    /// 调用者必须拥有 HWND，保证事务结束前它不被销毁/替换，且旧合成树已安全解绑。
    /// SetWindowLongPtrW/SetWindowPos 会重入窗口过程，禁止跨调用持有 Canvas/State 的借用。
    pub unsafe fn begin(hwnd: HWND, pinned: bool) -> Result<Self> {
        Self::begin_checked(hwnd, pinned, false, None)
    }
    /// # Safety
    /// 初始 HWND 必须归调用者；guard 必须验证稳定的宿主身份，不仅是 IsWindow。
    /// 重入销毁后 guard 失效，事务停止后续写入且不向可能复用的 HWND 回滚。
    pub unsafe fn begin_guarded(
        hwnd: HWND,
        pinned: bool,
        owns: &'a dyn Fn() -> bool,
    ) -> Result<Self> {
        Self::begin_checked(hwnd, pinned, false, Some(owns))
    }
    /// # Safety
    /// 仅在旧合成 owner 已确认退休且释放、新 GPU 尚未关联时调用；不跨调用持有 Canvas 借用。
    /// 必须在创建 GPU 候选前 commit，避免候选关联失败后回滚为 layered 并绕过关联门禁。
    pub unsafe fn gpu_guarded(hwnd: HWND, owns: &'a dyn Fn() -> bool) -> Result<Self> {
        Self::begin_checked(hwnd, false, true, Some(owns))
    }
    unsafe fn begin_checked(
        hwnd: HWND,
        pinned: bool,
        gpu: bool,
        owns: Option<&'a dyn Fn() -> bool>,
    ) -> Result<Self> {
        admitted(owns)?;
        let original = style(hwnd)?;
        // 分层提交不要求移除 NOREDIRECTIONBITMAP；普通 BitBlt 仍要求重定向表面。
        // 永不修改该标记：原地移除实测失败且可能部分写入其他位。
        if !gpu && !pinned && original & WS_EX_NOREDIRECTIONBITMAP.0 != 0 {
            return Err(Error::new(
                E_INVALIDARG,
                "Software presentation requires a replacement redirection-enabled HWND",
            ));
        }
        let original = original & MASK;
        let desired = if pinned { WS_EX_LAYERED.0 } else { 0 };
        let mut transaction = Self {
            hwnd,
            original,
            desired,
            active: true,
            owns,
            _thread: PhantomData,
        };
        if let Err(error) = replace_bits(hwnd, desired, owns) {
            if let Err(rollback) = transaction.rollback() {
                return Err(Error::new(
                    error.code(),
                    format!("Software window style failed: {error}; rollback={rollback}"),
                ));
            }
            return Err(error);
        }
        Ok(transaction)
    }

    pub fn commit(mut self) -> Result<()> {
        admitted(self.owns)?;
        if unsafe { style(self.hwnd)? } & MASK != self.desired {
            return Err(Error::new(
                E_FAIL,
                "Software window style changed before commit",
            ));
        }
        self.active = false;
        Ok(())
    }

    pub fn removes_layered(&self) -> bool {
        self.original & WS_EX_LAYERED.0 != 0 && self.desired & WS_EX_LAYERED.0 == 0
    }

    pub fn rollback(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        // 一次回滚，不在 Drop 中对同一失败继续重试或递归发窗口消息。
        self.active = false;
        if admitted(self.owns).is_err() {
            return Ok(());
        }
        unsafe { replace_bits(self.hwnd, self.original, self.owns) }
    }
}

impl Drop for Transition<'_> {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            eprintln!("native-software-style-rollback: {error}");
        }
    }
}

unsafe fn style(hwnd: HWND) -> Result<u32> {
    if !IsWindow(Some(hwnd)).as_bool()
        || GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId()
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Software style requires an owned UI-thread HWND",
        ));
    }
    SetLastError(WIN32_ERROR(0));
    let value = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if value == 0 && GetLastError() != WIN32_ERROR(0) {
        return Err(Error::from_win32());
    }
    Ok(value as u32)
}

fn admitted(owns: Option<&dyn Fn() -> bool>) -> Result<()> {
    if owns.is_none_or(|owns| owns()) {
        Ok(())
    } else {
        Err(Error::new(E_ABORT, "Software style owner was revoked"))
    }
}
unsafe fn replace_bits(hwnd: HWND, bits: u32, owns: Option<&dyn Fn() -> bool>) -> Result<()> {
    admitted(owns)?;
    let current = style(hwnd)?;
    let next = (current & !MASK) | bits;
    if current != next {
        SetLastError(WIN32_ERROR(0));
        if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next as isize) == 0
            && GetLastError() != WIN32_ERROR(0)
        {
            return Err(Error::from_win32());
        }
        admitted(owns)?;
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )?;
        admitted(owns)?;
    }
    if style(hwnd)? & MASK != bits {
        return Err(Error::new(
            E_FAIL,
            "Software window style mutation was not retained",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "software_window_style_tests.rs"]
mod tests;
