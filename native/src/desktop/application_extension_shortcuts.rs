//! 主干扩展快捷键的实际消息入口；焦点输入控件保留文字编辑，跨窗口只保存 UnitId 而非 HWND。
use super::*;
use crate::{desktop, loom_extension_snapshot::Chord};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
unsafe fn editable(hwnd: HWND) -> bool {
    if hwnd.is_invalid() {
        return false;
    }
    let mut name = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut name).max(0) as usize;
    matches!(
        String::from_utf16_lossy(&name[..n]).as_str(),
        "Edit" | "ComboBox" | "ComboLBox"
    )
}
unsafe fn document_for(documents: &[DocumentWindow], hwnd: HWND) -> Option<&DocumentWindow> {
    let root = GetAncestor(hwnd, GA_ROOTOWNER);
    documents
        .iter()
        .find(|d| d.is_alive() && (d.window.0 == hwnd || d.window.0 == root))
}
impl Application {
    pub(super) unsafe fn sync_extension_shortcuts(&mut self) {
        let catalog = self.extensions.view.borrow().snapshot.clone();
        if let Err(error) = self.extension_shortcuts.sync(catalog) {
            eprintln!("native-extension-shortcuts: {error}");
        }
    }
    pub(super) unsafe fn translate_extension_shortcuts(&mut self, message: &MSG) -> bool {
        let global = message.message == WM_HOTKEY && message.hwnd.is_invalid();
        let definition = if global {
            self.extension_shortcuts
                .global(message.wParam, message.lParam)
        } else if matches!(message.message, WM_KEYDOWN | WM_SYSKEYDOWN) {
            if document_for(&self.documents, message.hwnd).is_none() || editable(message.hwnd) {
                return false;
            }
            self.extension_shortcuts.local(Chord {
                key: message.wParam.0 as u32,
                modifiers: u32::from(GetKeyState(VK_MENU.0 as i32) < 0)
                    | (u32::from(GetKeyState(VK_CONTROL.0 as i32) < 0) << 1)
                    | (u32::from(GetKeyState(VK_SHIFT.0 as i32) < 0) << 2)
                    | (u32::from(
                        GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0,
                    ) << 3),
            })
        } else {
            return false;
        };
        let Some(definition) = definition.cloned() else {
            return false;
        };
        if !global && message.lParam.0 & (1 << 30) != 0 {
            return true;
        }
        if self.session_end.paused() || self.modals.blocked() || self.capture_active.get() {
            return global;
        }
        let document = if global {
            let target = self.extension_shortcuts.target.borrow();
            self.documents
                .iter()
                .find(|d| d.is_alive() && Some(&d.state.unit_id) == target.as_ref())
        } else {
            document_for(&self.documents, message.hwnd)
        };
        let Some(document) = document else {
            return global;
        };
        let state = &document.state;
        if !IsWindowEnabled(document.window.0).as_bool()
            || state.group_hidden.get()
            || !desktop::tool_panel_availability::Availability::capture(state)
                .is_some_and(|a| a.extension_allowed())
            || global
                && document_for(&self.documents, GetForegroundWindow()).is_some()
                && editable(GetFocus())
        {
            return global;
        }
        let Some(catalog) = &self.extension_shortcuts.catalog else {
            return global;
        };
        if !state
            .extensions
            .borrow()
            .snapshot
            .as_ref()
            .is_some_and(|s| std::sync::Arc::ptr_eq(s, catalog))
            || !desktop::extension_command_route::Context::capture(state)
                .with(|c| catalog.shortcut_available(&definition.id, c))
        {
            return global;
        }
        *self.extension_shortcuts.target.borrow_mut() = Some(state.unit_id.clone());
        if let Err(error) = desktop::extension_commands::request_command(
            document.window.0,
            state,
            &definition.command,
        ) {
            eprintln!("native-extension-shortcut-command: {error}");
        }
        true
    }
}
#[cfg(test)]
#[path = "application_extension_shortcut_tests.rs"]
mod tests;
