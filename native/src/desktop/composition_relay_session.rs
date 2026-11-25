//! UI-thread remote-viewing owner; close display first, then deliver terminal state and release thread handles after background completion.
use super::window::RelayWindow;
use crate::{
    byte_budget::Budget,
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    live_relay_control_runtime::{Exit, Runtime, Target},
    live_relay_viewer_identity::ControlEvent,
};
use std::{io, os::windows::io::BorrowedHandle, sync::Arc};

pub struct Outcome {
    pub exit: Exit,
    pub window_error: Option<String>,
}
impl Outcome {
    pub fn error_message(&self) -> Option<String> {
        self.window_error.clone().or_else(|| match self.exit {
            Exit::Stopped
            | Exit::Closed
            | Exit::Media(
                crate::live_relay_viewer_runtime::Exit::Stopped
                | crate::live_relay_viewer_runtime::Exit::Closed,
            ) => None,
            other => Some(format!("{other:?}")),
        })
    }
}
pub struct Session {
    runtime: Runtime,
    window: Option<RelayWindow>,
    visible: bool,
    stopping: bool,
    completed: bool,
    ready_seen: bool,
    window_error: Option<String>,
}
impl Session {
    pub(super) fn take_local_trigger(&self) -> Option<crate::art_live_trigger::ticket::Ticket> {
        if self.stopping || self.completed {
            return None;
        }
        self.runtime.take_local_trigger()
    }
    #[cfg(test)]
    pub(super) fn test_disconnect_control(&self) -> bool {
        self.runtime.test_disconnect_control()
    }
    #[cfg(test)]
    pub(super) fn test_control(&self) -> (bool, u64) {
        self.runtime.test_control()
    }
    #[cfg(test)]
    pub(super) fn test_disconnect_media(&self) -> bool {
        self.runtime.test_disconnect_media()
    }
    #[cfg(test)]
    pub(super) fn test_observation(
        &self,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        windows::Win32::Foundation::HWND,
    )> {
        self.window.as_ref().and_then(RelayWindow::observation_test)
    }
    #[cfg(test)]
    pub(super) fn test_input(
        &self,
    ) -> Option<(
        windows::Win32::Foundation::HWND,
        crate::live_relay_control_runtime::interaction::Status,
    )> {
        self.window
            .as_ref()
            .map(|window| (window.input_test_hwnd(), window.input_test_status()))
    }
    #[cfg(test)]
    pub(crate) fn test_frame(
        &self,
    ) -> windows::core::Result<Option<(crate::live_relay_frame::Metadata, [u8; 4])>> {
        self.window
            .as_ref()
            .map(RelayWindow::test_frame)
            .transpose()
            .map(Option::flatten)
    }
    /// Start background tasks only, without showing windows; consumer handles real business events and must not silently ignore them.
    pub fn start(
        config: Config,
        lease: Lease,
        target: Target,
        budget: Arc<Budget>,
        consume: impl FnMut(&[ControlEvent]) -> Result<(), ()> + Send + 'static,
    ) -> io::Result<Self> {
        Runtime::start_viewer_configured(config, lease, target, budget, consume)
            .map(Self::from_runtime)
    }
    pub(crate) fn from_runtime(runtime: Runtime) -> Self {
        Self {
            runtime,
            window: None,
            visible: false,
            stopping: false,
            completed: false,
            ready_seen: false,
            window_error: None,
        }
    }
    /// Record user visibility intent even before Ready; show must not revive stopped sessions.
    pub fn show(&mut self) -> windows::core::Result<()> {
        if self.stopping || self.completed {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_FAIL,
                "Remote session stopped",
            ));
        }
        self.visible = true;
        if let Some(window) = &self.window {
            window.show()?;
        }
        Ok(())
    }
    pub fn has_window(&self) -> bool {
        self.window.as_ref().is_some_and(RelayWindow::is_alive)
    }
    pub(super) fn joined(&self) -> bool {
        self.ready_seen
    }
    pub(super) fn stopping(&self) -> bool {
        self.stopping
    }
    /// Callers must keep Session alive while waiting and must not concurrently poll/stop and destroy window handles.
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        let ready = (!self.stopping && !self.ready_seen && !self.completed)
            .then(|| self.runtime.ready_handle());
        [
            ready,
            self.runtime.wait_handle(),
            self.window.as_ref().and_then(RelayWindow::change_handle),
        ]
        .into_iter()
        .flatten()
    }
    pub fn stop(&mut self) {
        self.stopping = true;
        self.visible = false;
        self.window.take();
        self.runtime.request_stop();
    }
    /// Call after message dispatch or handle wakeup; return Outcome once, only after actual thread join completes.
    pub fn poll(&mut self) -> Option<Outcome> {
        if self.completed {
            return None;
        }
        if let Some(exit) = self.runtime.try_join() {
            return Some(self.finish(exit));
        }
        if !self.stopping && !self.ready_seen {
            if let Some(ready) = self.runtime.try_ready() {
                self.ready_seen = true;
                let media = ready.media.clone();
                let permit = ready.permit.clone();
                let result =
                    RelayWindow::new(ready.media, ready.permit, 960, 640).and_then(|window| {
                        window.bind_input(ready.input);
                        if self.visible {
                            window.show()?;
                        }
                        Ok(window)
                    });
                match result {
                    Ok(window) => self.window = Some(window),
                    Err(error) if media.validate(&permit).is_ok() => {
                        self.window_error = Some(error.to_string());
                        self.stop();
                    }
                    Err(_) => {} // The permit is invalid: retain background terminal state rather than recording revocation as window-construction failure.
                }
            }
        }
        if let Some(window) = &self.window {
            if let Err(error) = window.refresh() {
                self.window_error.get_or_insert_with(|| error.to_string());
            }
            if let Some(error) = window.take_error() {
                self.window_error.get_or_insert_with(|| error.to_string());
            }
            if self.window_error.is_some() {
                self.stop();
            } else if !window.is_alive() {
                // Media-permit invalidation actively closes windows; await original background terminal state instead of preemptively cancelling and rewriting the reason.
                let user_closed = window.is_authorized();
                self.window.take();
                if user_closed {
                    self.stop();
                }
            }
        }
        self.runtime.try_join().map(|exit| self.finish(exit))
    }
    fn finish(&mut self, exit: Exit) -> Outcome {
        self.completed = true;
        self.window.take();
        Outcome {
            exit,
            window_error: self.window_error.take(),
        }
    }
    #[cfg(test)]
    pub(crate) fn close_window_for_test(&self) {
        self.window.as_ref().unwrap().close().unwrap();
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        // Normal application exit must first pump/wait and poll; Runtime Drop is the blocking leak-prevention fallback.
    }
}
