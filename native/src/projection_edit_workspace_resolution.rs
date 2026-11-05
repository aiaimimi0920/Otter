//! 显式冲突选择只改变矢量视图；read期间的新编辑重读/保留，底图或会话换代一律拒绝。
use super::*;
impl Snapshot {
    pub(crate) fn rejected(&self, code: Code) -> Result<Self, Error> {
        if !code.recoverable() || self.suspended.is_some() {
            return Err(Error::ContentChanged);
        }
        let value = Self {
            conflict: Some(code),
            ..self.clone()
        };
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn resolved(
        &self,
        document: Document,
        observed: &View,
        current: &View,
        keep_local: bool,
    ) -> Result<Self, Error> {
        if self.suspended.is_some()
            || self.conflict.is_none()
            || (keep_local
                && self.role == Role::Receiver
                && document.mode() != super::super::Mode::TwoWay)
        {
            return Err(Error::ContentChanged);
        }
        let later =
            super::super::view::reconcile(&self.document, observed, &document, current, None)
                .map_err(|_| Error::Identity)?;
        let visible = if keep_local {
            let baseline =
                View::from_document(&self.document, current.size()).map_err(|_| Error::Format)?;
            let changes = baseline.changes_to(current).map_err(|_| Error::Format)?;
            later
                .authoritative()
                .overlay(&changes)
                .map_err(|_| Error::Format)?
        } else {
            later.visible().clone()
        };
        let conflict = if self.role == Role::Receiver
            && document.mode() == super::super::Mode::OneWay
            && !later.changes().is_empty()
        {
            Some(Code::ReadOnly)
        } else {
            later.conflict().map(|v| v.code())
        };
        let value = Self {
            document,
            visible: visible.basis_objects().clone(),
            conflict,
            ..self.clone()
        };
        value.validate()?;
        Ok(value)
    }
}
#[cfg(test)]
#[path = "projection_edit_workspace_resolution_tests.rs"]
mod tests;
