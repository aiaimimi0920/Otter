//! Pair window owners with capture workers; windows unsubscribe first, and real thread handles wake the host for reclamation after completion.
use super::{window::LiveWindow, Texture};
use crate::live_job::LiveJob;
use std::{io, os::windows::io::BorrowedHandle};
use windows::Win32::Foundation::RECT;
#[path = "composition_live_session_input.rs"]
mod input;
#[path = "composition_live_pin_session.rs"]
mod pin;
#[path = "composition_live_popup_family.rs"]
mod popup_family;
#[path = "composition_live_remote_input.rs"]
mod remote_input;
#[path = "composition_live_source.rs"]
mod source;

pub struct Session {
    pub(in crate::desktop) naming: Option<crate::desktop::capture_window_meta::Meta>,
    popups: popup_family::Family,
    popup_scope: Option<crate::live_window_target::popup::Scope>,
    pin_document: Option<crate::desktop::document_window::DocumentWindow>,
    pin_attached: bool,
    input_identity: std::sync::Arc<()>,
    unit_id: crate::workspace_id::UnitId,
    workspace: Option<crate::workspace::IdentityStamp>,
    window: Option<LiveWindow>,
    job: LiveJob<Texture>,
    error: Option<String>,
    stopping: bool,
    capture_result: Option<Result<(), String>>,
    target: Option<crate::live_window_target::Target>,
    input_crop: Option<crate::live_window_target::region::Crop>,
    input:
        Option<crate::live_input_worker::Worker<crate::live_window_target::input_backend::Error>>,
    input_enabled: bool,
    input_error: Option<String>,
    input_transition: bool,
    input_cleanup: Option<crate::live_input::Cleanup>,
    input_cleanup_unknown: bool,
    remote_input: Option<remote_input::Remote>,
    last_input_sequence: u64,
    protection: Option<crate::live_visibility_owner::Protection>,
}
impl Session {
    pub(in crate::desktop) fn bind_workspace_chrome(
        &self,
        binding: std::rc::Weak<crate::desktop::application::chrome::Ui>,
    ) {
        if let Some(window) = &self.window {
            window.bind_workspace_chrome(binding);
        }
    }
    /// The user has confirmed the window target; start background capture only after presentation-window exclusion succeeds.
    pub fn start_window(
        target: crate::live_window_target::Target,
        screen: RECT,
    ) -> io::Result<Self> {
        Self::start_window_region(target, None, screen)
    }
    pub fn start_window_region(
        target: crate::live_window_target::Target,
        crop: Option<crate::live_window_target::region::Crop>,
        screen: RECT,
    ) -> io::Result<Self> {
        let (job, window) = LiveJob::start_window_region_bound(target.clone(), crop, |consumer| {
            LiveWindow::new_at(consumer, screen).map_err(io::Error::other)
        })?;
        let mut session = Self::new(job, window);
        if let Some(window) = &session.window {
            window.allow_input().map_err(io::Error::other)?;
            if !crate::live_browser_scroll::eligible(&target) {
                window.popup_watch().map_err(io::Error::other)?;
            }
        }
        session.target = Some(target);
        session.input_crop = crop;
        if let Some(window) = &session.window {
            window.request_default_input();
        }
        Ok(session)
    }
    /// Only for user-confirmed physical selections; do not display the window, leaving explicit show to the caller.
    pub fn start_region(screen: RECT) -> io::Result<Self> {
        let (job, window) = LiveJob::start_region_bound(screen, |consumer| {
            LiveWindow::new_at(consumer, screen).map_err(io::Error::other)
        })?;
        Ok(Self::new(job, window))
    }
    pub(in crate::desktop) fn new(job: LiveJob<Texture>, window: LiveWindow) -> Self {
        Self {
            popups: Default::default(),
            naming: None,
            popup_scope: None,
            pin_document: None,
            pin_attached: false,
            input_identity: std::sync::Arc::new(()),
            unit_id: window.unit_id().clone(),
            workspace: None,
            window: Some(window),
            job,
            error: None,
            stopping: false,
            capture_result: None,
            target: None,
            input_crop: None,
            input: None,
            input_enabled: false,
            input_error: None,
            input_transition: false,
            input_cleanup: None,
            input_cleanup_unknown: false,
            remote_input: None,
            last_input_sequence: 0,
            protection: None,
        }
    }
    pub fn show(&self) -> windows::core::Result<()> {
        self.window
            .as_ref()
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_FAIL,
                    "Live session window is closed",
                )
            })?
            .show()
    }
    pub fn unit_id(&self) -> &crate::workspace_id::UnitId {
        &self.unit_id
    }
    pub(in crate::desktop) fn input_identity(&self) -> std::sync::Arc<()> {
        self.input_identity.clone()
    }
    pub(in crate::desktop) fn input_identity_ref(&self) -> &std::sync::Arc<()> {
        &self.input_identity
    }
    pub(in crate::desktop) fn bind_workspace(&mut self, workspace: &crate::workspace::Workspace) {
        if self.workspace.is_none() {
            self.workspace = Some(workspace.identity_stamp());
            if let Some(window) = &self.window {
                window.enable_recycle_close();
            }
        }
    }
    pub(in crate::desktop) fn belongs_to(&self, workspace: &crate::workspace::Workspace) -> bool {
        self.workspace
            .as_ref()
            .is_some_and(|stamp| stamp.matches(workspace))
    }
    pub fn is_alive(&self) -> bool {
        !self.stopping && self.window.as_ref().is_some_and(LiveWindow::is_alive)
    }
    pub(in crate::desktop) fn persistence_idle(&self) -> bool {
        self.is_alive()
            && self
                .window
                .as_ref()
                .is_some_and(LiveWindow::persistence_idle)
    }
    pub(in crate::desktop) fn opacity(&self) -> Option<f32> {
        self.is_alive()
            .then(|| self.window.as_ref().unwrap().opacity())
    }
    pub(in crate::desktop) fn persistence_state(&self) -> Option<(RECT, f32)> {
        self.window
            .as_ref()
            .filter(|_| self.is_alive())?
            .persistence_state()
    }
    pub(in crate::desktop) fn exit_idle(&self) -> bool {
        self.is_alive() && self.window.as_ref().is_some_and(LiveWindow::exit_idle)
    }
    pub fn take_freeze_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_freeze_request)
    }
    pub(in crate::desktop) fn take_snapshot_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_snapshot_request)
    }
    pub(in crate::desktop) fn take_link_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_link_request)
    }
    pub(in crate::desktop) fn take_art_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_art_request)
    }
    pub(in crate::desktop) fn art_pending(&self) -> bool {
        self.window.as_ref().is_some_and(LiveWindow::art_pending)
    }
    pub(in crate::desktop) fn art_dialog_owner(&self) -> Option<windows::Win32::Foundation::HWND> {
        self.window
            .as_ref()
            .filter(|_| !self.stopping)?
            .art_dialog_owner()
    }
    pub(in crate::desktop) fn take_unlink_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_unlink_request)
    }
    pub(in crate::desktop) fn take_group_request(&self) -> Option<super::window::groups::Request> {
        self.window
            .as_ref()
            .and_then(LiveWindow::take_group_request)
    }
    pub(in crate::desktop) fn group_input_blocked(&self) -> bool {
        self.stopping
            || self
                .window
                .as_ref()
                .is_none_or(LiveWindow::group_input_blocked)
    }
    pub(in crate::desktop) fn group_dialog_owner(
        &self,
    ) -> Option<windows::Win32::Foundation::HWND> {
        if self.stopping {
            return None;
        }
        self.window
            .as_ref()
            .and_then(LiveWindow::group_dialog_owner)
    }
    pub(in crate::desktop) fn take_input_notice(&self) -> Option<windows::Win32::Foundation::HWND> {
        self.window
            .as_ref()
            .filter(|window| window.take_input_notice())
            .and_then(LiveWindow::group_dialog_owner)
    }
    pub(in crate::desktop) fn sync_groups(
        &self,
        workspace: &crate::workspace::Workspace,
    ) -> windows::core::Result<()> {
        if self.belongs_to(workspace) {
            if let Some(window) = &self.window {
                window.sync_groups(workspace)?;
            }
        }
        Ok(())
    }
    pub fn take_recycle_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_recycle_request)
    }
    pub(in crate::desktop) fn close_for_recycle(&mut self) -> windows::core::Result<()> {
        self.window
            .as_ref()
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Live recycle session closed",
                )
            })?
            .close_for_recycle()?;
        self.stop();
        Ok(())
    }
    /// Explicit pixel boundary; do not stop capture/input, and the application transaction still owns commits after successful conversion.
    pub fn snapshot_frame(
        &self,
    ) -> windows::core::Result<Option<std::sync::Arc<crate::live_frames::Frame<Texture>>>> {
        match self.window.as_ref().filter(|_| !self.stopping) {
            Some(window) => window.snapshot_frame(),
            None => Ok(None),
        }
    }
    pub fn freeze_rect(
        &self,
        source: [u32; 2],
    ) -> windows::core::Result<crate::pin_geometry::PinRect> {
        self.window
            .as_ref()
            .filter(|_| !self.stopping)
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Live session closed",
                )
            })?
            .freeze_rect(source)
    }
    pub(crate) fn protect(&mut self, protection: crate::live_visibility_owner::Protection) {
        self.protection = Some(protection);
    }
    /// Add to the host message-wait set; source close may precede thread exit, but this handle cannot report completion early.
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.job
            .completion_handle()
            .or_else(|| self.popups.completion_handle())
    }
    pub fn source_process_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.job.source_process_handle()
    }
    /// Explicit stop never joins in UI callbacks; retain owners until poll collects results or final application destruction.
    pub fn stop(&mut self) {
        self.popups.stop();
        self.pin_document.take();
        self.stopping = true;
        self.cancel_remote_input();
        self.disable_input();
        // Attached mirrors may still release source keys/cancel modals; parent-source protection must cover the whole group's input cleanup.
        if self.input_completion_handle().is_none() {
            self.protection.take();
        }
        self.window.take();
        self.job.cancel();
    }
    /// Call after each message dispatch and when thread handles become ready; B's bounded low-frequency timer wakes attached-popup discovery.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        if let Some(window) = &self.window {
            window
                .resolve_popup_focus(self.target.as_ref().filter(|_| self.remote_input.is_none()));
        }
        self.poll_popups();
        self.poll_input();
        self.poll_remote_input();
        self.process_window_input();
        if let Some(window) = &self.window {
            // poll_popups has revalidated popup scope; provenance observation runs neither in HWND callbacks nor remote paths.
            window.refresh_ime(
                self.target.as_ref().filter(|_| self.remote_input.is_none()),
                self.input_crop,
            );
        }
        self.job.refresh_source();
        if let Some(window) = &self.window {
            if let Err(error) = window.refresh() {
                self.error.get_or_insert_with(|| error.to_string());
            }
            if let Some(error) = window.take_error() {
                self.error.get_or_insert_with(|| error.to_string());
            }
            if !window.is_alive() || self.error.is_some() {
                // Do not actively cancel: when source failure closes the window, preserve the worker's original error.
                self.window.take();
                self.cancel_remote_input();
                self.disable_input();
            }
        }
        if self.capture_result.is_none() {
            self.capture_result = self.job.take_ready();
        }
        if self.capture_result.is_some() {
            self.window.take();
            self.cancel_remote_input();
            self.disable_input();
        }
        if self.window.is_none() && self.input_completion_handle().is_none() {
            self.protection.take();
        }
        if self.capture_result.is_none() {
            // Do not hide sources after output closes or tasks finish, preventing late ACKs from causing hide/restore flicker.
            if let Some(protection) = self.protection.as_ref().filter(|_| self.window.is_some()) {
                if let Err(error) = protection.poll() {
                    self.error.get_or_insert_with(|| error.to_string());
                    self.window.take();
                    self.disable_input();
                    if self.input_completion_handle().is_none() {
                        self.protection.take();
                    }
                    self.job.cancel();
                }
            }
            return None;
        }
        if self.input.is_some() || !self.popups.is_empty() {
            return None;
        }
        if self.input_cleanup_unconfirmed() {
            self.error
                .get_or_insert_with(|| "Live input cleanup failed or outcome unknown".into());
        }
        let result = self.capture_result.take()?;
        self.protection.take();
        self.window.take();
        Some(if let Some(error) = self.error.take() {
            Err(error)
        } else if self.stopping {
            Ok(())
        } else {
            result
        })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        // LiveJob's final destructor still joins; detaching must not hide uninterruptible driver calls.
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_input_backpressure_tests.rs"]
mod input_backpressure_tests;
#[cfg(test)]
#[path = "composition_live_input_runtime_tests.rs"]
mod input_runtime_tests;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_session_tests.rs"]
mod tests;
