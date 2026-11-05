//! 软件帧离开 Canvas 借用作用域后提交；按 target 身份和文档凭证确认，不把旧代帧记入新文档。
use super::Canvas;
use std::rc::Rc;
use windows::{core::Result, Win32::Foundation::HWND};

pub(in crate::desktop) struct Pending {
    frame: super::super::software_surface::Submission,
    identity: Rc<()>,
    document: crate::editor::DocumentStamp,
    confirm: bool,
    pub(in crate::desktop) pinned: bool,
}
pub(in crate::desktop) struct Presented {
    identity: Rc<()>,
    document: crate::editor::DocumentStamp,
    confirm: bool,
}
impl Pending {
    pub fn present(self, hwnd: HWND) -> Result<Presented> {
        self.frame.present(hwnd)?;
        Ok(Presented {
            identity: self.identity,
            document: self.document,
            confirm: self.confirm,
        })
    }
}
impl Canvas {
    pub(in crate::desktop) fn take_submission(&mut self, normal_document: bool) -> Option<Pending> {
        let target = self.target.as_mut()?;
        Some(Pending {
            frame: target.surface.take_submission()?,
            identity: Rc::clone(&target.identity),
            document: self.editor.document_stamp(),
            confirm: normal_document,
            pinned: self.pinned,
        })
    }
    pub(in crate::desktop) fn accepts_pending(&self, pending: &Pending) -> bool {
        self.target
            .as_ref()
            .is_some_and(|target| Rc::ptr_eq(&target.identity, &pending.identity))
            && pending.document.matches(&self.editor)
    }
    pub(in crate::desktop) fn accepts_presented(&self, presented: &Presented) -> bool {
        self.target
            .as_ref()
            .is_some_and(|target| Rc::ptr_eq(&target.identity, &presented.identity))
            && presented.document.matches(&self.editor)
    }
    pub(in crate::desktop) fn confirm_submission(&mut self, presented: Presented) -> bool {
        if !self.accepts_presented(&presented) {
            return false;
        }
        if presented.confirm {
            self.confirmed_paints = self.confirmed_paints.saturating_add(1);
        }
        true
    }
}
