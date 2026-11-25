//! Local Live operations borrow an ordinary pin host; one logical owner per ID, with the capture thread alive throughout editing.
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

impl Application {
    pub(in crate::desktop::application) unsafe fn commit_live_pin(
        &mut self,
        unit: crate::workspace_id::UnitId,
        source: crate::session_source::Source,
        pin: crate::desktop::open_image::PreparedDocument,
        key: (usize, isize, [u8; 256]),
        visible: bool,
    ) -> Result<()> {
        if self.exit_requested.get()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || !self
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&source))
            || self.documents.iter().any(|doc| doc.state.unit_id == unit)
        {
            return Err(Error::new(E_ABORT, "Live pin workspace changed"));
        }
        let index = self
            .live_sessions
            .iter()
            .position(|session| {
                session.unit_id() == &unit
                    && session.is_alive()
                    && !session.group_input_blocked()
                    && session.belongs_to(&self.workspace)
            })
            .ok_or_else(|| Error::new(E_ABORT, "Live pin source closed"))?;
        let frame = self.live_sessions[index].freeze_rect([pin.image.width, pin.image.height])?;
        let opacity = self.live_sessions[index]
            .opacity()
            .ok_or_else(|| Error::new(E_ABORT, "Live pin opacity missing"))?;
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve Live pin"))?;
        let document = if let Some(doc) = self.live_sessions[index].pin_document() {
            // On failure, Session still owns the old document; do not take its owner before a fallible window operation.
            SetWindowPos(
                doc.window.0,
                None,
                frame.x,
                frame.y,
                frame.width as i32,
                frame.height as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
            let mut canvas = doc.state.canvas.borrow_mut();
            let refreshed = pin
                .editable
                .as_ref()
                .ok_or_else(|| Error::new(E_ABORT, "Live edit snapshot missing"))?;
            if !canvas.editor.replace_source(pin.image.pixels.clone()) {
                let revision = canvas.editor.revision();
                if !canvas
                    .editor
                    .adopt_snapshot(revision, &refreshed.editor)
                    .map_err(|error| {
                        Error::new(E_FAIL, format!("Live edit refresh failed: {error:?}"))
                    })?
                {
                    return Err(Error::new(E_ABORT, "Live edit changed during refresh"));
                }
            }
            canvas.sync_raster_source();
            drop(canvas);
            self.live_sessions[index].take_pin_document().unwrap()
        } else {
            let mut state = State::from_prepared(pin)?;
            state.unit_id = unit.clone();
            state.live_pin.set(true);
            *state.execution_results.borrow_mut() =
                crate::desktop::loom_result_state::Results::new(unit.clone());
            self.prepare_window_at(state, false, frame)?
        };
        {
            let mut canvas = document.state.canvas.borrow_mut();
            canvas.normal_opacity = opacity;
            canvas.window_opacity = opacity;
            canvas.live_pin = true;
        }
        self.live_sessions[index].attach_pin();
        let hwnd = document.window.0;
        self.documents.push(document);
        self.sync_capture_windows();
        self.sync_groups();
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
        }
        // Even if the first key is released before background readback, use its key-down modifiers, then restore the thread keyboard state.
        let mut prior = [0; 256];
        GetKeyboardState(&mut prior)?;
        SetKeyboardState(&key.2)?;
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(key.0)), Some(LPARAM(key.1)));
        SetKeyboardState(&prior)?;
        self.saved_state = None;
        Ok(())
    }

    pub(in crate::desktop::application) unsafe fn settle_live_pins(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        let mut index = 0;
        while index < self.documents.len() {
            let doc = &self.documents[index];
            if !doc.state.live_pin.get() {
                index += 1;
                continue;
            }
            let session = self
                .live_sessions
                .iter()
                .position(|session| session.unit_id() == &doc.state.unit_id);
            if !doc.is_alive() {
                if let Some(session) = session {
                    self.live_sessions[session].stop();
                }
                index += 1;
                continue;
            }
            let Some(session) = session.filter(|s| self.live_sessions[*s].is_alive()) else {
                // The source has exited; retain the current edit snapshot as an ordinary pin.
                doc.state.live_pin.set(false);
                doc.state.canvas.borrow_mut().live_pin = false;
                index += 1;
                continue;
            };
            if !doc.state.live_resume.replace(false) {
                index += 1;
                continue;
            }
            if !doc.state.canvas.borrow().blocks_wheel()
                && !crate::desktop::group_lock::editing_busy(&doc.state)
                && !self.modals.blocked()
                && !doc.state.group_locked.get()
                && !doc.state.group_hidden.get()
                && !self.session_end.paused()
                && self.save_job.is_none()
                && self.restore_job.is_none()
            {
                self.live_sessions[session].prepare_pin_return(doc)?;
                let doc = self.documents.remove(index);
                self.live_sessions[session].retain_pin(doc, visible);
                self.sync_capture_windows();
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}
