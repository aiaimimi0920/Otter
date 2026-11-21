//! 输入 worker 固定原弹窗来源；输入权限与原实例的收尾资格分离，不从当前 HWND 重建授权。
use super::*;
use crate::live_window_target::popup::Transfer;
impl Backend {
    /// 仅在 worker 构建时消费 UI 快照；远程不能取得本地弹窗来源能力。
    pub(crate) fn with_popup_scope(mut self, transfer: Option<Transfer>) -> Result<Self, Error> {
        if transfer.is_some() && !self.local {
            return Err(Error::InvalidDelivery);
        }
        self.popup_scope = transfer.map(Transfer::into_scope);
        self.check_popup_scope()?;
        self.modal.set_popup_scope(
            self.popup_scope
                .as_ref()
                .map(|scope| scope.native_snapshot())
                .transpose()
                .map_err(|error| Error::PopupScope(error.to_string()))?
                .unwrap_or_default(),
        );
        Ok(self)
    }
    pub(super) fn check_popup_scope(&self) -> Result<(), Error> {
        if let Some(scope) = &self.popup_scope {
            let target = scope
                .target()
                .map_err(|error| Error::PopupScope(error.to_string()))?;
            if !target.same_source(&self.root)
                || !unsafe {
                    windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(target.hwnd())
                }
                .as_bool()
            {
                return Err(Error::PopupScope(
                    "Live popup source changed or hidden".into(),
                ));
            }
        }
        Ok(())
    }
    pub(super) fn popup_cleanup_allowed(&self) -> bool {
        self.popup_scope.as_ref().is_none_or(|scope| {
            scope
                .cleanup_target()
                .is_ok_and(|target| target.same_source(&self.root))
        })
    }
}
#[cfg(test)]
#[path = "live_input_popup_backend_tests.rs"]
mod tests;
