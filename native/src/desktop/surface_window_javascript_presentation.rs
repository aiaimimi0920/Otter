//! 服务端收敛只暂存权威快照；真实同 child update/DOM 交付之前不报告呈现成功。
use super::*;
impl AuthorizedWindow {
    pub(in crate::desktop::surface_authorized_window) fn stage_javascript_snapshot(
        &mut self,
        resources: crate::surface_resource_snapshot::ResourceSnapshot,
        revision: u64,
        now: u64,
    ) -> std::result::Result<Option<u64>, PresentationError> {
        let current = resources
            .mount(self.binding(), revision)
            .map_err(|_| PresentationError::Evidence)?;
        let js = self
            .javascript
            .as_ref()
            .ok_or(PresentationError::Evidence)?;
        if !self.javascript_ready() || js.events.is_some() || js.presentation.is_some() {
            return Err(PresentationError::Evidence);
        }
        if revision == self.snapshot_revision {
            let old = self.javascript_mount().map_err(PresentationError::Window)?;
            return if old.snapshot.get() == current.snapshot.get() {
                Ok(Some(revision))
            } else {
                Err(PresentationError::Revision)
            };
        }
        if revision < self.snapshot_revision {
            return Err(PresentationError::Revision);
        }
        current
            .javascript_entry(now)
            .map_err(|_| PresentationError::Evidence)?;
        self.window
            .set_submission_enabled(false)
            .map_err(PresentationError::Window)?;
        let viewport = self
            .capture_javascript_viewport()
            .map_err(PresentationError::Window)?;
        let js = self.javascript.as_mut().unwrap();
        js.runtime
            .as_mut()
            .ok_or(PresentationError::Evidence)?
            .update(current, viewport)
            .map_err(|error| PresentationError::Window(Error::new(E_FAIL, error.to_string())))?;
        js.staged = Some((resources, revision));
        js.presentation = Some(revision);
        Err(PresentationError::Pending(revision))
    }
    pub(crate) fn finish_javascript_presentation(
        &mut self,
        binding: &Binding,
        revision: u64,
        now: u64,
    ) -> std::result::Result<Option<u64>, PresentationError> {
        self.refresh(now).map_err(PresentationError::Window)?;
        self.lease
            .validate(binding.origin(), now)
            .map_err(PresentationError::Authorization)?;
        if !self.is_shown() {
            return Err(PresentationError::Inactive);
        }
        if self.binding() != binding
            || self.javascript.as_ref().and_then(|js| js.presentation) != Some(revision)
        {
            return Err(PresentationError::Evidence);
        }
        if !self.javascript_ready() {
            return Err(PresentationError::Pending(revision));
        }
        if self.snapshot_revision != revision {
            return Err(PresentationError::Revision);
        }
        self.javascript
            .as_ref()
            .unwrap()
            .runtime
            .as_ref()
            .unwrap()
            .hold_notification(false)
            .map_err(|error| PresentationError::Window(Error::new(E_FAIL, error.to_string())))?;
        self.javascript.as_mut().unwrap().presentation.take();
        Ok(Some(revision))
    }
}
