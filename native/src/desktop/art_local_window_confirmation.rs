//! 原窗口拥有提示；领取批准前重验当前宿主、授权和可见策略，关闭时默认拒绝。
use super::*;
impl Window {
    pub(in crate::desktop) fn confirmation_handle(
        &self,
    ) -> Option<windows::Win32::Foundation::HWND> {
        self.confirmation
            .as_ref()
            .and_then(confirmation_dialog::Prompt::handle)
    }
    pub(super) fn poll_confirmation(
        &mut self,
        host: &impl Fn() -> bool,
        visible: bool,
    ) -> Result<()> {
        if !visible || !self.current(host) {
            self.confirmation.take();
            if let Some(ticket) = self.runtime.take_confirmation(&self.current) {
                ticket.resolve(false);
            }
            return Ok(());
        }
        if let Some(choice) = self
            .confirmation
            .as_ref()
            .and_then(|prompt| prompt.take_choice())
        {
            let approved = choice && visible && self.current(host);
            if let Some(prompt) = self.confirmation.take() {
                prompt.resolve(approved);
            }
        }
        if self.confirmation.is_none() {
            if let Some(ticket) = self.runtime.take_confirmation(&self.current) {
                let hwnd = self
                    .window
                    .as_ref()
                    .ok_or_else(|| Error::new(E_FAIL, "Local Art host closed"))?
                    .dialog_owner()?;
                match confirmation_dialog::Prompt::new(hwnd, ticket.clone()) {
                    Ok(prompt) => self.confirmation = Some(prompt),
                    Err(_) => {
                        ticket.resolve(false);
                    }
                }
            }
        }
        Ok(())
    }
}
