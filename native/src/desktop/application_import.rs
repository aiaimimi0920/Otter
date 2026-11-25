//! Prepare all batch windows before adding them to the application together; capacity/window-creation failures release staged owners without showing a partial batch.
use super::*;
#[path = "application_paste_target.rs"]
mod paste_target;

impl Application {
    pub(super) unsafe fn add_prepared(
        &mut self,
        pins: Vec<super::super::open_image::PreparedDocument>,
        visible: bool,
    ) -> Result<()> {
        self.add_prepared_with(pins, visible, |app, state, observe, frame| {
            if let Some(frame) = frame {
                app.prepare_window_at(state, observe, frame)
            } else {
                app.prepare_window(state, observe)
            }
        })
    }
    pub(super) unsafe fn add_prepared_with(
        &mut self,
        pins: Vec<super::super::open_image::PreparedDocument>,
        visible: bool,
        mut prepare: impl FnMut(
            &Self,
            State,
            bool,
            Option<crate::pin_geometry::PinRect>,
        ) -> Result<DocumentWindow>,
    ) -> Result<()> {
        if pins.is_empty()
            || pins.len() > super::super::open_job::MAX_BATCH
            || self.documents.len().saturating_add(pins.len()) > crate::workspace::MAX_UNITS
        {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Native import window budget exhausted",
            ));
        }
        self.documents
            .try_reserve(pins.len())
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve imported windows"))?;
        let mut staged = Vec::new();
        staged
            .try_reserve_exact(pins.len())
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve staged windows"))?;
        let mut placements = Vec::new();
        placements
            .try_reserve_exact(pins.len())
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve clipboard placements"))?;
        let presentation: Vec<_> = pins
            .iter()
            .map(|pin| {
                (
                    pin.record_history,
                    !pin.record_history && pin.paste_origin.is_none(),
                )
            })
            .collect();
        for mut pin in pins {
            let request = pin
                .placement
                .take()
                .map(|request| self.editable_clipboard.borrow().refresh(&request));
            let frame = request
                .as_ref()
                .map(|request| self.paste_frame(request))
                .transpose()?;
            let origin = pin.paste_origin;
            let document = prepare(self, State::from_prepared(pin)?, visible, frame)?;
            if let Some(cursor) = origin.filter(|_| frame.is_none()) {
                // Position only hidden staged windows; do not borrow capture_origin's 1:1 screenshot-size semantics.
                SetWindowPos(
                    document.window.0,
                    None,
                    cursor.x,
                    cursor.y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )?;
            }
            staged.push(document);
            placements.push(request.zip(frame));
        }
        let start = self.documents.len();
        self.workspace
            .add_units(staged.iter().map(|document| document.state.unit_id.clone()))
            .map_err(|error| {
                Error::new(
                    E_FAIL,
                    format!("Cannot register imported workspace units: {error:?}"),
                )
            })?;
        self.documents.extend(staged);
        for (document, (captured, _)) in self.documents[start..].iter().zip(&presentation) {
            if *captured {
                crate::desktop::capture_history_ui::record(&document.state);
            }
        }
        // Only committed units advance the anchor; cancelled/failed staging is invisible.
        for (document, placement) in self.documents[start..].iter().zip(placements) {
            if let Some((request, frame)) = placement {
                self.editable_clipboard.borrow_mut().placed(
                    &request,
                    document.state.unit_id.clone(),
                    frame,
                );
            }
        }
        self.sync_capture_windows();
        if visible {
            for (document, (_, show_panel)) in self.documents[start..].iter().zip(presentation) {
                let _ = ShowWindow(document.window.0, SW_SHOWNORMAL);
                // Screenshots and pasted images default to free dragging; ordinary file imports retain the original display rules.
                if show_panel {
                    let _ = ShowWindow(document.panel.0, SW_SHOWNOACTIVATE);
                }
            }
        }
        Ok(())
    }
}
