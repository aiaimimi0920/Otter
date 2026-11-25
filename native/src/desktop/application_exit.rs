//! Save the default scene on application exit without prompting for manual save-as; retain windows on failure and release owners only after commit.
use super::*;

#[cfg_attr(
    not(any(test, debug_assertions, feature = "smoke-test")),
    allow(dead_code)
)]
pub(super) enum Choice {
    Save,
    Discard,
    Cancel,
}

/// Hidden HWNDs verify decisions and completion-state transitions without showing dialogs or writing the user session.
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
pub(super) unsafe fn verify(app: &mut Application) -> Result<()> {
    let saved = app.saved_state.take();
    let count = app.documents.len();
    app.exit_requested.set(true);
    if app.close_requested_with(|_| Choice::Cancel)
        || app.exit_requested.get()
        || app.documents.len() != count
    {
        return Err(Error::new(
            E_FAIL,
            "Cancelled exit changed document ownership",
        ));
    }
    app.exit_requested.set(true);
    if app.close_requested_with(|_| Choice::Save) || !app.exit_after_save {
        return Err(Error::new(E_FAIL, "Save before exit did not queue"));
    }
    for document in &app.documents {
        document.state.save_existing_requested.set(false);
    }
    app.finish_exit_save(false);
    if app.exit_after_save || app.exit_requested.get() {
        return Err(Error::new(E_FAIL, "Failed save retained exit intent"));
    }
    let opacity = app.documents[0].state.canvas.borrow().normal_opacity;
    app.exit_requested.set(true);
    if app.close_requested_with(|app| {
        app.documents[0].state.canvas.borrow_mut().normal_opacity =
            if opacity == 0.5 { 0.6 } else { 0.5 };
        Choice::Discard
    }) {
        return Err(Error::new(E_FAIL, "Late modal change was discarded"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    app.saved_state = Some(save_state::Stamp::capture(app)?);
    app.documents[0].state.canvas.borrow_mut().normal_opacity =
        if opacity == 0.5 { 0.6 } else { 0.5 };
    app.exit_after_save = true;
    app.finish_exit_save(true);
    if app.exit_requested.get() || app.exit_after_save {
        return Err(Error::new(E_FAIL, "Changed workspace exited after save"));
    }
    app.documents[0].state.canvas.borrow_mut().normal_opacity = opacity;
    app.exit_after_save = true;
    app.finish_exit_save(true);
    if !app.exit_requested.replace(false) || app.exit_after_save {
        return Err(Error::new(
            E_FAIL,
            "Unchanged saved workspace did not resume exit",
        ));
    }
    app.saved_state = saved;
    app.exit_saved = None;
    println!("native-exit-save-smoke: cancel-preserved=true save-queued=true failure-preserved=true changed-preserved=true clean-resumed=true");
    Ok(())
}

impl Application {
    pub(super) unsafe fn close_requested(&mut self) -> bool {
        self.close_requested_automatically_with_notify(|app, message| {
            if let Some(doc) = app.documents.iter().find(|doc| doc.is_alive()) {
                app.save_message(&doc.state.unit_id, message, true);
            }
        })
    }

    pub(super) unsafe fn close_requested_automatically_with_notify(
        &mut self,
        notify: impl FnOnce(&Self, &str),
    ) -> bool {
        if !self.modals.blocked()
            && self.exit_requested.get()
            && self.save_job.is_none()
            && self.checkpoint_writes_stopped()
            && self.exit_saved.is_none()
            && !self
                .saved_state
                .as_ref()
                .is_some_and(|stamp| stamp.matches_automatic(self))
        {
            self.exit_requested.set(false);
            notify(self, crate::ui_text::text("app-exit-autosave-unavailable"));
            return false;
        }
        // Discard skips only the manual save-as file; the default scene's final checkpoint must still complete afterward.
        self.close_requested_with_notify(|_| Choice::Discard, notify)
    }
    #[cfg(any(test, debug_assertions, feature = "smoke-test"))]
    pub(super) unsafe fn close_requested_with(
        &mut self,
        choose: impl FnOnce(&Self) -> Choice,
    ) -> bool {
        self.close_requested_with_notify(choose, |app, message| {
            if let Some(doc) = app.documents.iter().find(|doc| doc.is_alive()) {
                app.save_message(&doc.state.unit_id, message, true);
            }
        })
    }
    pub(super) unsafe fn close_requested_with_notify(
        &mut self,
        choose: impl FnOnce(&Self) -> Choice,
        notify: impl FnOnce(&Self, &str),
    ) -> bool {
        if self.modals.blocked() {
            return false;
        }
        if let Some(ready) = self.finish_checkpoint_exit() {
            return ready && self.close_confirmed();
        }
        if !self.exit_requested.get() {
            return false;
        }
        if self.naming_autosave_blocks_exit()
            || self.locale_blocks_exit()
            || self.workspace_autosave_blocks_exit()
            || self.checkpoint_pending()
        {
            return false;
        }
        self.exit_requested.set(false);
        self.open_after_save = None;
        let approved = self.exit_saved.take();
        if approved
            .as_ref()
            .is_some_and(|intent| !intent.matches(self))
        {
            return false;
        }
        // Do not insert another confirmation or destroy the source window while saving; recheck state after completion.
        if self.save_job.is_some() {
            // A task already owned by the save UI may continue exiting after completion; failure still clears the exit intent.
            self.exit_after_save = self.save_origin.is_some();
            return false;
        }
        if self.exit_after_save {
            return false;
        }
        if approved.is_none() && self.save_status() != save_state::Status::Unchanged {
            let Ok(before) = exit_intent::Intent::capture(self) else {
                return false;
            };
            match choose(self) {
                Choice::Cancel => return false,
                Choice::Save => {
                    if let Some(document) = self.documents.iter().find(|doc| doc.is_alive()) {
                        self.exit_after_save =
                            save_ui::request_existing(document.window.0, &document.state).is_ok();
                    }
                    return false;
                }
                // Window/background messages may arrive during the modal interval; discard intent must not extend to subsequently changed content.
                Choice::Discard if !before.matches(self) => return false,
                Choice::Discard => {}
            }
        }
        let result = exit_intent::Intent::capture(self).and_then(|intent| {
            if !intent.matches(self) {
                return Err(Error::new(
                    E_ABORT,
                    crate::ui_text::text("app-exit-operations-pending"),
                ));
            }
            self.begin_checkpoint_exit(intent)
        });
        match result {
            Ok(true) => false,
            Ok(false) => self.close_confirmed(),
            Err(error) => {
                notify(
                    self,
                    &crate::ui_text::format(
                        "app-exit-checkpoint-failed",
                        &[("error", error.to_string().into())],
                    ),
                );
                false
            }
        }
    }
    unsafe fn close_confirmed(&mut self) -> bool {
        self.exit_committed = true;
        self.checkpoint_exiting();
        // Saving/checkpointing has completed; from here, driver or restore waits must not leave a windowless host resident forever.
        crate::emergency_watchdog::shutdown::begin();
        for document in &self.documents {
            if document.is_alive() {
                SendMessageW(document.window.0, WM_CLOSE, None, None);
            }
        }
        self.collect_closed();
        true
    }

    pub(super) unsafe fn finish_exit_save(&mut self, success: bool) {
        if !std::mem::take(&mut self.exit_after_save)
            || !success
            || !self
                .saved_state
                .as_ref()
                .is_some_and(|stamp| stamp.matches_automatic(self))
        {
            return;
        }
        let Ok(intent) = exit_intent::Intent::capture(self) else {
            return;
        };
        if !intent.matches(self) {
            return;
        }
        // Do not destroy windows on the save-result handling stack; resume normal exit checks on the next message turn.
        if PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0)).is_ok() {
            self.exit_saved = Some(intent);
            self.exit_requested.set(true);
        }
    }
}
