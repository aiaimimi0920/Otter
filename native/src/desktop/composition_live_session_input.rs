//! Session input owner: explicit enabling and asynchronous settlement; do not retire source protection before input release completes.
use super::Session;
use crate::{
    live_input::{Event, MAX_SEQUENCE},
    live_input_worker::{Command, Reason, Worker},
    live_window_target::input_backend::Backend,
};
use std::{io, os::windows::io::BorrowedHandle};
impl Session {
    pub(in crate::desktop) fn resume_minimized_source(
        &mut self,
        owner: &mut crate::live_visibility_owner::Owner,
    ) {
        if self.stopping
            || self.remote_input.is_some()
            || self.capture_result.is_some()
            || self.window.as_ref().is_none_or(|w| {
                !w.is_alive() || !w.source_activation_requested() || w.group_input_blocked()
            })
        {
            return;
        }
        let Some(protection) = &self.protection else {
            return;
        };
        if !protection.recalled()
            || self.target.as_ref().is_none_or(|target| unsafe {
                !windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd()).as_bool()
            })
        {
            return;
        }
        match owner.resume_minimized(protection) {
            Ok(_) => {
                self.disable_input();
                if let Some(window) = &self.window {
                    window.defer_input_resume();
                }
            }
            Err(error) => {
                self.input_error.get_or_insert_with(|| error.to_string());
                self.disable_input();
                if let Some(window) = &self.window {
                    window.input_failure();
                }
            }
        }
    }

    pub(super) fn process_window_input(&mut self) {
        if self.remote_input.is_some() {
            // The same worker still handles network input, but remote eligibility must not become local-window mouse/keyboard control.
            if let Some(window) = &self.window {
                window.take_input_request();
                while window.pop_input().is_some() {}
                window.input_status(false, false, None);
            }
            return;
        }
        if self.target.as_ref().is_some_and(|target| unsafe {
            windows::Win32::UI::WindowsAndMessaging::IsIconic(target.hwnd()).as_bool()
        }) {
            // Background recovery initiated by B must await a new journal ACK, without clearing the queued first input.
            if self.protection.is_some()
                && self
                    .window
                    .as_ref()
                    .is_some_and(|w| w.input_resume_pending())
            {
                return;
            }
            if let Some(window) = &self.window {
                window.suspend_source_input();
            }
            self.disable_input();
        }
        let request = self
            .window
            .as_ref()
            .and_then(|window| window.take_input_request());
        if let Some(enabled) = request {
            if enabled {
                if let Err(error) = self.enable_input() {
                    self.input_error.get_or_insert_with(|| error.to_string());
                }
            } else {
                self.disable_input();
            }
        }
        // Keep only one input in flight; later slow-source moves stay coalesced in the window queue instead of filling the worker prematurely.
        // Replies wake continued sending through original WM_NULL; critical edges remain FIFO, and cancellation bypasses this admission condition.
        while self.input_enabled
            && self
                .input
                .as_ref()
                .is_some_and(|worker| !worker.stopping() && worker.outstanding() == 0)
        {
            let Some(event) = self.window.as_ref().and_then(|window| {
                window.pop_dispatch_input(|| {
                    self.target
                        .as_ref()
                        .is_some_and(crate::live_browser_scroll::eligible)
                })
            }) else {
                break;
            };
            let sequence = self.last_input_sequence.saturating_add(1);
            if let Err(error) = self.send_input(sequence, event) {
                self.input_error.get_or_insert_with(|| error.to_string());
                self.disable_input();
                break;
            }
        }
        if let Some(window) = &self.window {
            let pending = !self.input_enabled
                && (window.input_resume_pending()
                    || self.input.as_ref().is_some_and(|worker| !worker.stopping()));
            let stop = self.input.as_ref().and_then(Worker::stop_signal);
            window.input_status(self.input_enabled, pending, stop);
            if self.input_error.is_some() && !self.input_enabled && !pending {
                window.input_failure();
            }
        }
    }
    pub fn enable_input(&mut self) -> io::Result<()> {
        if self.remote_input.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Remote Live owns input",
            ));
        }
        self.start_input(true)
    }
    // The remote bridge temporarily takes Remote while handling requests; an empty field does not imply local input.
    pub(super) fn start_input(&mut self, local: bool) -> io::Result<()> {
        if self
            .window
            .as_ref()
            .is_some_and(|window| window.group_input_blocked())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Live group is locked or hidden",
            ));
        }
        if self.stopping
            || self.window.as_ref().is_none_or(|window| !window.is_alive())
            || self.capture_result.is_some()
        {
            return Err(io::Error::other("Live session is closing"));
        }
        if self.input.is_some() {
            if self.input.as_ref().is_some_and(|worker| !worker.stopping()) {
                return Ok(());
            }
            // The original owner still drains releases after pause; retain one recovery intent without stacking workers.
            self.input_enabled = false;
            if let Some(window) = &self.window {
                window.defer_input_resume();
            }
            return Ok(());
        }
        if self.input_cleanup_unknown || self.input_cleanup.is_some_and(|report| report.failed > 0)
        {
            return Err(io::Error::other("Live input release is unconfirmed"));
        }
        let target = self.target.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Region Live cannot control a window",
            )
        })?;
        let region = self.input_crop.map(|crop| crop.input());
        // Publish the pending cycle before factory can run, or epoch remains zero during default first-time enable.
        if local {
            if let Some(window) = &self.window {
                window.input_status(false, true, None);
            }
        }
        let hover = self
            .window
            .as_ref()
            .filter(|_| local)
            .and_then(|window| window.hover_identity())
            .map(|identity| identity.capture_control())
            .transpose()
            .map_err(io::Error::other)
            .inspect_err(|_| {
                if let Some(window) = &self.window {
                    window.input_failure();
                }
            })?;
        let create = if local {
            #[cfg(test)]
            let local = Backend::new_local_in_test_harness;
            #[cfg(not(test))]
            let local = Backend::new_local;
            local
        } else {
            Backend::new
        };
        let popup = self
            .popup_scope
            .as_ref()
            .filter(|_| local)
            .map(|scope| scope.transfer());
        let mut worker = Worker::start_for_ui(move || {
            create(target, region)
                .map(|backend| backend.with_hover(hover))
                .and_then(|backend| backend.with_popup_scope(popup))
                .map_err(|error| format!("{error:?}"))
        })
        .inspect_err(|_| {
            if let Some(window) = &self.window {
                window.input_failure();
            }
        })?;
        let submitted = worker.submit(Command::Enable);
        self.input = Some(worker);
        self.input_enabled = false;
        self.input_error = None;
        self.input_transition = false;
        self.input_cleanup = None;
        self.input_cleanup_unknown = false;
        if let Err(error) = submitted {
            self.disable_input();
            if let Some(window) = &self.window {
                window.input_failure();
            }
            return Err(io::Error::other(format!("{error:?}")));
        }
        Ok(())
    }
    pub fn input_enabled(&self) -> bool {
        self.input_enabled
    }
    pub fn take_input_error(&mut self) -> Option<String> {
        self.input_error.take()
    }
    pub fn input_cleanup(&self) -> Option<crate::live_input::Cleanup> {
        self.input_cleanup
    }
    pub(super) fn input_cleanup_unconfirmed(&self) -> bool {
        self.input_cleanup_unknown
            || self.input_cleanup.is_some_and(|report| report.failed > 0)
            || self.popups.input_cleanup_unconfirmed
    }
    pub fn input_completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.input
            .as_ref()
            .and_then(Worker::completion_handle)
            .or_else(|| self.popups.input_completion_handle())
    }
    pub fn send_input(&mut self, sequence: u64, event: Event) -> io::Result<()> {
        if !self.input_enabled
            || self
                .window
                .as_ref()
                .is_some_and(|window| window.group_input_blocked())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Live interaction not enabled",
            ));
        }
        if sequence == 0 || sequence > MAX_SEQUENCE || sequence <= self.last_input_sequence {
            self.disable_input();
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Live input sequence rejected",
            ));
        }
        let result = self
            .input
            .as_mut()
            .ok_or_else(|| io::Error::other("Live input owner missing"))?
            .submit(Command::Input { sequence, event });
        match result {
            Ok(()) => {
                self.last_input_sequence = sequence;
                Ok(())
            }
            Err(error) => {
                self.disable_input();
                Err(io::Error::other(format!("{error:?}")))
            }
        }
    }
    pub fn disable_input(&mut self) {
        self.input_enabled = false;
        if let Some(worker) = &mut self.input {
            worker.stop();
        }
    }
    pub(super) fn poll_input(&mut self) {
        let Some(worker) = &mut self.input else {
            return;
        };
        // Observe thread exit before draining replies so simultaneous final errors and exit cannot lose classification.
        let exit = worker.take_exit();
        loop {
            match worker.poll_reply() {
                Ok(Some(reply)) => {
                    match reply.result {
                        Ok(crate::live_input::Delivery::Pending) => {
                            // Release only this submission's admission placeholder, without claiming window-procedure completion; subsequent menu input may proceed.
                        }
                        Ok(crate::live_input::Delivery::IgnoredWhileModal) => {
                            // Duplicate right-click was not delivered; preserve the existing modal input channel instead of revoking control as a failure.
                        }
                        Ok(crate::live_input::Delivery::Completed) => {
                            if matches!(reply.command, Some(Command::Enable))
                                && !worker.stopping()
                                && !self.stopping
                            {
                                self.input_enabled = true;
                            }
                        }
                        Err(error) => {
                            use crate::live_window_target::{
                                input_backend, input_coordinates, input_message,
                            };
                            let transition = self.remote_input.is_none()
                                && matches!(
                                    error,
                                    crate::live_input::Error::Backend(
                                        input_backend::Error::Coordinates(
                                            input_coordinates::Error::SourceTransition
                                        ) | input_backend::Error::BrowserTransition(_)
                                            | input_backend::Error::Message(
                                                input_message::Error::SourceTransition
                                            )
                                    )
                                );
                            let revoked = self.remote_input.is_none()
                                && matches!(
                                    error,
                                    crate::live_input::Error::Backend(
                                        input_backend::Error::ControlRevoked
                                            | input_backend::Error::Message(
                                                input_message::Error::ControlRevoked
                                            )
                                    )
                                );
                            if revoked {
                                // Confirmed unpublished/unsent; preserve UI intent for the new cycle so the old worker cannot pause it.
                                eprintln!("native-live-input-pause: reason=control-cycle detail={error:?}");
                                self.input_transition = true;
                            } else if transition {
                                eprintln!("native-live-input-pause: reason=source-transition detail={error:?}");
                                // Minimization or browser-query transitions confirmed to have performed no action are recoverable; never replay failed input.
                                self.input_transition = true;
                                if let Some(window) = &self.window {
                                    window.suspend_source_input();
                                    if matches!(
                                        error,
                                        crate::live_input::Error::Backend(
                                            input_backend::Error::BrowserTransition(_)
                                        )
                                    ) {
                                        // A query with no action may restore admission; no new owner is created before the old worker finishes releasing.
                                        window.resume_browser_input();
                                    }
                                }
                            } else {
                                self.input_error.get_or_insert_with(|| format!("{error:?}"));
                            }
                            self.input_enabled = false;
                            worker.stop();
                        }
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    self.input_error.get_or_insert_with(|| error.to_string());
                    self.input_enabled = false;
                    worker.stop();
                    break;
                }
            }
        }
        if let Some(exit) = exit {
            self.input_cleanup = exit.cleanup;
            self.input_cleanup_unknown = exit.cleanup.is_none();
            if exit.cleanup.is_none_or(|report| report.failed > 0)
                && (self.stopping || self.window.as_ref().is_none_or(|window| !window.is_alive()))
            {
                self.error
                    .get_or_insert_with(|| "Live input cleanup failed or outcome unknown".into());
            }
            self.input_enabled = false;
            if exit.reason != Reason::Stopped
                && !(self.input_transition && exit.reason == Reason::InputFailed)
            {
                self.input_error
                    .get_or_insert_with(|| format!("Live input stopped: {:?}", exit.reason));
            }
            match exit.cleanup {
                Some(report) if report.failed > 0 => {
                    self.input_error.get_or_insert_with(|| {
                        format!("Live input release failed: {}", report.failed)
                    });
                }
                None => {
                    self.input_error
                        .get_or_insert_with(|| "Live input cleanup outcome unknown".into());
                }
                _ => {}
            }
            if self.input_error.is_some() {
                if let Some(window) = &self.window {
                    window.input_failure();
                }
            }
            self.input = None;
            self.input_transition = false;
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_input_epoch_tests.rs"]
mod epoch_tests;
