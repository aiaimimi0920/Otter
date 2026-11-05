//! 与主干一致的逻辑隐藏；仅由已 arm 的共享 lease 调用，不使用 SW_HIDE 或延时 sleep。
use super::*;
impl Snapshot {
    pub(crate) fn apply_hidden(&self) -> Result<()> {
        self.target.validate()?;
        unsafe {
            let hwnd = self.target.hwnd();
            // 先登记原始最小化快照，再恢复可采集窗口；不能在 ACK 前更改源。
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                self.target.validate()?;
                if IsIconic(hwnd).as_bool() {
                    return Err(Error::new(E_FAIL, "Live source remained minimized"));
                }
            }
            let style =
                self.extended_style | WS_EX_LAYERED.0 as isize | WS_EX_TRANSPARENT.0 as isize;
            self.target.validate()?;
            SetLastError(ERROR_SUCCESS);
            let previous = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
            if previous == 0 && GetLastError() != ERROR_SUCCESS {
                return Err(Error::from_win32());
            }
            self.target.validate()?;
            if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & style != style {
                return Err(Error::new(
                    E_FAIL,
                    "Live source rejected transparency style",
                ));
            }
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA)?;
            self.target.validate()?;
            SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
            self.target.validate()
        }
    }
}
