//! Live 采集 owner 保留普通钉图编辑历史；编辑宿主交给 Application 时不重复登记节点。
use super::*;
use crate::desktop::document_window::DocumentWindow;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

impl Session {
    pub(in crate::desktop) fn take_pin_request(&self) -> Option<(usize, isize, [u8; 256])> {
        self.window.as_ref()?.take_pin_request()
    }
    pub(in crate::desktop) fn pin_attached(&self) -> bool {
        self.pin_attached
    }
    pub(in crate::desktop) fn take_pin_document(&mut self) -> Option<DocumentWindow> {
        self.pin_document.take()
    }
    pub(in crate::desktop) fn pin_document(&self) -> Option<&DocumentWindow> {
        self.pin_document.as_ref()
    }
    pub(in crate::desktop) fn pin_snapshot(&self) -> Option<crate::editor::Editor> {
        Some(
            self.pin_document
                .as_ref()?
                .state
                .canvas
                .borrow()
                .editor
                .export_snapshot(),
        )
    }
    pub(in crate::desktop) fn attach_pin(&mut self) {
        self.pin_attached = true;
        if let Some(window) = &self.window {
            window.pin_editing(true);
        }
    }
    pub(in crate::desktop) unsafe fn prepare_pin_return(&self, doc: &DocumentWindow) -> Result<()> {
        let window = self
            .window
            .as_ref()
            .ok_or_else(|| windows::core::Error::new(E_ABORT, "Live closed during edit"))?;
        let mut rect = RECT::default();
        GetWindowRect(doc.window.0, &mut rect)?;
        let opacity = doc.state.canvas.borrow().window_opacity;
        window.apply_pin(
            &rect,
            opacity,
            doc.state.canvas.borrow().editor.export_snapshot(),
        )?;
        Ok(())
    }
    /// 所有可失败准备都在文档仍归 Application 时完成，之后只转移 owner。
    pub(in crate::desktop) unsafe fn retain_pin(&mut self, doc: DocumentWindow, visible: bool) {
        let _ = ShowWindow(doc.panel.0, SW_HIDE);
        let _ = ShowWindow(doc.window.0, SW_HIDE);
        self.pin_document = Some(doc);
        self.pin_attached = false;
        if let Some(window) = &self.window {
            if visible {
                window.pin_editing(false);
            }
            window.resume_source();
        }
    }
}
