//! Window recovery exclusively owns the retiring Target; ordinary discard/image replacement must not lose retirement gates or authorize new associations.
use super::{Canvas, Target};
use crate::desktop::composition_retirement::{Stage, Status};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_FAIL},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::desktop) enum Destination {
    Automatic,
    #[default]
    Compatibility,
    Software,
}

#[derive(Default)]
pub(super) struct Recovery {
    pub(super) target: Option<Target>,
    pub(super) status: Option<Status>,
    destination: Destination,
}

impl Canvas {
    pub(in crate::desktop) fn software_selected(&self) -> bool {
        self.software_renderer
    }
    #[cfg(test)]
    pub(in crate::desktop) fn render_choice(&self) -> Destination {
        if self.software_renderer {
            Destination::Software
        } else if self.compatibility_renderer {
            Destination::Compatibility
        } else {
            Destination::Automatic
        }
    }
    pub(in crate::desktop) fn needs_gpu_style(&self) -> bool {
        self.target.is_none() && !self.software_renderer
    }
    /// User choices cannot clear creation rejection or unconfirmed retirement; terminal state permits rebuilding only owners known to have no composition graph.
    pub(in crate::desktop) fn can_request_render(&self) -> bool {
        if self.creation_blocked() || self.recovery.status == Some(Status::Pending) {
            return false;
        }
        self.recovery.status.is_none()
            || (self.software_renderer && self.recovery.target.is_none())
            || (self.recovery.target.is_none() && self.creation_admission.retryable_failure())
            || self
                .recovery
                .target
                .as_ref()
                .is_some_and(|target| target.surface.is_software())
    }
    /// true means the window timer must still finish retirement; accepting a request is not presentation and cannot clear the original error.
    pub(in crate::desktop) fn request_render(&mut self, destination: Destination) -> Result<bool> {
        if !self.can_request_render() {
            return Err(Error::new(
                E_ABORT,
                "Render association has not authorized a manual retry",
            ));
        }
        if self.recovery.status.is_some() {
            self.recovery.target.take();
            self.recovery.status = None;
        }
        if self.target.as_ref().is_some_and(|target| {
            matches!(
                target.surface,
                super::RenderSurface::D3d12(_) | super::RenderSurface::D3d11(_)
            )
        }) {
            self.begin_recovery(destination);
            return Ok(true);
        }
        self.discard_device();
        self.select_render(destination);
        Ok(false)
    }
    fn select_render(&mut self, destination: Destination) {
        self.compatibility_renderer = destination != Destination::Automatic;
        self.software_renderer = destination == Destination::Software;
    }
    pub(in crate::desktop) fn creation_blocked(&self) -> bool {
        self.creation_admission.blocked()
    }
    pub(in crate::desktop) fn render_suspended(&self) -> bool {
        self.recovery.status.is_some()
    }
    pub(super) fn ensure_render_admitted(&self) -> Result<()> {
        if self.render_suspended() {
            Err(Error::new(
                E_ABORT,
                "Render recovery has not authorized a new frame",
            ))
        } else {
            Ok(())
        }
    }
    /// Explicit self-test retirement does not initiate window recovery; callers still stop drawing and retain Canvas.
    pub(in crate::desktop) fn retire_composition(
        &mut self,
    ) -> Option<crate::desktop::composition_retirement::Status> {
        self.target
            .as_mut()
            .and_then(|target| target.surface.retire_composition())
    }
    pub(in crate::desktop) fn recover_render(&mut self) -> bool {
        if self.render_suspended() || self.software_renderer {
            return false;
        }
        let Some(target) = &self.target else {
            return false;
        };
        let destination = if !self.compatibility_renderer && target.surface.requires_compatibility()
        {
            Destination::Compatibility
        } else if target.surface.requires_software() {
            Destination::Software
        } else {
            return false;
        };
        self.begin_recovery(destination);
        true
    }
    pub(in crate::desktop) fn recover_render_after(
        &mut self,
        _code: windows::core::HRESULT,
    ) -> bool {
        // HRESULT alone is not DComp retirement evidence; both GPU backends start recovery from their actual owner's failure state.
        self.recover_render()
    }
    fn begin_recovery(&mut self, destination: Destination) {
        // After transferring the complete owner, minimize, source changes, and ordinary discard touch only the active target.
        self.recovery.target = self.target.take();
        self.recovery.destination = destination;
        self.recovery.status = Some(Status::Pending);
    }
    /// CPU targets without DComp associations retain one rebuild attempt; neither GPU backend may use it to bypass retirement.
    pub(in crate::desktop) fn retry_render_without_composition(&mut self) -> bool {
        if self.render_suspended()
            || self.target.as_ref().is_some_and(|target| {
                matches!(
                    target.surface,
                    super::RenderSurface::D3d12(_) | super::RenderSurface::D3d11(_)
                )
            })
        {
            return false;
        }
        self.discard_device();
        true
    }
    pub(in crate::desktop) fn poll_render_recovery(&mut self) -> Option<Status> {
        if self.recovery.status != Some(Status::Pending) {
            return self.recovery.status;
        }
        let status = self
            .recovery
            .target
            .as_mut()
            .and_then(|target| target.surface.retire_composition())
            .unwrap_or(Status::Failed {
                stage: Stage::Worker,
                code: E_FAIL,
            });
        self.resolve_recovery(status);
        Some(status)
    }
    fn resolve_recovery(&mut self, status: Status) {
        // Late success cannot unlock terminal window state or republish a compatibility candidate.
        if self.recovery.status != Some(Status::Pending) {
            return;
        }
        if status == Status::Complete {
            // Complete confirms only DComp. The old owner's Drop still independently applies GPU drain/retention policy.
            self.recovery.target.take();
            self.select_render(self.recovery.destination);
            self.recovery.status = None;
        } else {
            self.recovery.status = Some(status);
        }
    }
    /// Scheduling failure or unrecoverable drawing errors preserve the document and stop retries; failure does not mean closing the window.
    pub(in crate::desktop) fn suspend_render(&mut self) {
        if self.recovery.target.is_none() {
            self.recovery.target = self.target.take();
        }
        if let Some(target) = self.recovery.target.as_mut() {
            let _ = target.surface.retire_composition();
        }
        self.recovery.status = Some(Status::Failed {
            stage: Stage::Worker,
            code: E_ABORT,
        });
    }
    pub(in crate::desktop) fn close_render(&mut self) {
        // Do not continue after HWND destruction; the backend's retaining Drop handles unconfirmed graphs, never joining system-wait threads.
        self.recovery.status = Some(Status::Failed {
            stage: Stage::Worker,
            code: E_ABORT,
        });
        self.recovery.target.take();
        self.discard_device();
    }
}

#[cfg(test)]
#[path = "canvas_render_recovery_tests.rs"]
mod tests;
