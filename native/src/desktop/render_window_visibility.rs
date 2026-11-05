//! CPU 分层内容→GPU 的可见性重建；临时隐藏不改变激活、位置或层序，所有写入受稳定宿主约束。
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

pub(super) struct Reset<'a> {
    hwnd: HWND,
    restore: bool,
    owns: &'a dyn Fn() -> bool,
    allow_restore: &'a dyn Fn() -> bool,
    _thread: PhantomData<Rc<()>>,
}

impl<'a> Reset<'a> {
    /// # Safety
    /// HWND 必须属于本线程；旧 GPU 已退休，新 GPU 未关联。不得跨调用持有 Canvas 借用。
    /// removed_layered 只能来自本次已提交的样式事务，不能对普通 GPU 重绘反复隐藏。
    pub(super) unsafe fn begin(
        hwnd: HWND,
        removed_layered: bool,
        owns: &'a dyn Fn() -> bool,
        allow_restore: &'a dyn Fn() -> bool,
    ) -> Result<Self> {
        if !owns() {
            return Err(Error::new(E_ABORT, "Render visibility owner was revoked"));
        }
        let reset = Self {
            hwnd,
            restore: removed_layered && IsWindowVisible(hwnd).as_bool(),
            owns,
            allow_restore,
            _thread: PhantomData,
        };
        if reset.restore {
            // NOACTIVATE 实测保留本线程 active/focus 和其他应用前台；SW_HIDE 不提供此合同。
            position(hwnd, SWP_HIDEWINDOW)?;
            if !owns() {
                return Err(Error::new(E_ABORT, "Render visibility owner was revoked"));
            }
        }
        Ok(reset)
    }

    pub(super) unsafe fn restore(&mut self) -> Result<()> {
        // 一次尝试；失败不在 Drop 中刷重试，也不覆盖重入产生的分组/托盘隐藏意图。
        if !std::mem::take(&mut self.restore) || !(self.owns)() || !(self.allow_restore)() {
            return Ok(());
        }
        position(self.hwnd, SWP_SHOWWINDOW)?;
        // SHOW 自身也会重入；外层 SHOW 不能覆盖其中新产生的隐藏意图。
        // 仅纠正一次，不与重入业务循环争夺可见性；销毁后绝不补写。
        if (self.owns)() && !(self.allow_restore)() {
            position(self.hwnd, SWP_HIDEWINDOW)?;
        }
        Ok(())
    }
}

unsafe fn position(hwnd: HWND, visibility: SET_WINDOW_POS_FLAGS) -> Result<()> {
    SetWindowPos(
        hwnd,
        None,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE | visibility,
    )
}

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        if let Err(error) = unsafe { self.restore() } {
            eprintln!("native-render-visibility-restore: {error}");
        }
    }
}

#[cfg(test)]
#[path = "render_window_visibility_tests.rs"]
mod tests;
