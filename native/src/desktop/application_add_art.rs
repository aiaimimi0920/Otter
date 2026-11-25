//! Sole publication transaction for Add Art intent; prepare windows and connections first, then recheck source/catalog/workspace before publishing them together.
use super::*;
use crate::desktop::{
    self,
    node_menu::art_picker::{Request, Route},
};
#[path = "application_live_add_art.rs"]
mod live;
#[path = "application_art_source.rs"]
pub(super) mod source;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "application_add_art_tests.rs"]
mod tests;
impl Application {
    pub(super) unsafe fn process_art_requests(&mut self, visible: bool) -> Result<()> {
        if self.modals.blocked() || self.session_end.paused() {
            return Ok(());
        }
        self.process_live_art_requests(visible)?;
        let requests: Vec<_> = self
            .documents
            .iter()
            .filter_map(|doc| {
                doc.state
                    .art_request
                    .borrow_mut()
                    .take()
                    .map(|request| (doc.state.unit_id.clone(), request))
            })
            .collect();
        for (unit, request) in requests {
            let result = self.add_connected_art(&unit, request);
            if let Err(error) = result {
                if !visible {
                    return Err(error);
                }
                if let Some(hwnd) = self.source_window(&unit) {
                    MessageBoxW(
                        Some(hwnd),
                        &HSTRING::from(crate::ui_text::format(
                            "app-art-not-added",
                            &[("error", error.to_string().into())],
                        )),
                        w!("Otter"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
        Ok(())
    }
    unsafe fn add_connected_art(
        &mut self,
        unit: &crate::workspace_id::UnitId,
        request: Request,
    ) -> Result<()> {
        if self.exit_requested.get()
            || self.session_end.paused()
            || self.modals.blocked()
            || self.restore_job.is_some()
            || self.save_job.is_some()
        {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-workspace-transition-busy"),
            ));
        }
        let source = self
            .documents
            .iter()
            .find(|doc| {
                &doc.state.unit_id == unit && doc.is_alive() && self.workspace.contains(unit)
            })
            .ok_or_else(|| {
                Error::new(
                    E_ABORT,
                    crate::ui_text::text("app-art-source-node-unavailable"),
                )
            })?;
        if !request.current(source.window.0, &source.state) {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-art-source-catalog-changed"),
            ));
        }
        let source = source::Source::capture(self, unit)?;
        let art = request
            .catalog
            .handshake
            .capability(request.index)
            .ok_or_else(|| Error::from(E_INVALIDARG))?;
        if let Route::Local(origin) = &request.route {
            // origin comes only from explicit HTTPS selection; the WSS catalog provides ID hints only, while the original local chain still validates actual packages/permissions.
            let config =
                crate::device_authorization_config::Config::from_env(origin).map_err(|_| {
                    Error::new(
                        E_INVALIDARG,
                        crate::ui_text::text("app-art-service-invalid"),
                    )
                })?;
            return self.open_local_art_child(
                origin,
                art.id.into(),
                unit.as_str(),
                config,
                |app| {
                    app.documents.iter().any(|source| {
                        &source.state.unit_id == unit
                            && request.current(source.window.0, &source.state)
                    })
                },
            );
        }
        self.create_backend_art(&source, request.catalog.clone(), request.index, |app| {
            app.documents
                .iter()
                .any(|doc| &doc.state.unit_id == unit && request.current(doc.window.0, &doc.state))
        })
    }
    pub(super) unsafe fn create_backend_art(
        &mut self,
        source: &source::Source,
        catalog: std::rc::Rc<desktop::capability_job::Loaded>,
        index: usize,
        current: impl Fn(&Self) -> bool,
    ) -> Result<()> {
        if !source.current(self) || !current(self) {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-art-source-changed"),
            ));
        }
        let art = catalog
            .handshake
            .capability(index)
            .filter(|art| art.enabled)
            .ok_or_else(|| {
                Error::new(
                    E_INVALIDARG,
                    crate::ui_text::text("app-art-capability-unavailable"),
                )
            })?;
        let parameters = crate::loom_parameter_state::Parameters::new(&art).map_err(|_| {
            Error::new(
                E_INVALIDARG,
                crate::ui_text::text("app-art-parameters-failed"),
            )
        })?;
        let input = desktop::node_menu::art_picker::model::primary_input(&art);
        let execution = art.execution_config(None);
        let guard = restore_batch::guard::Guard::capture(self)?;
        let (rect, visible) = source.geometry()?;
        let state = State::new(None, true)?;
        let child = state.unit_id.clone();
        *state.selected_art.borrow_mut() = Some(desktop::capability_selection::Selection {
            execution,
            identity: desktop::document_state::new_unit_id()?,
            catalog,
            parameters,
        });
        let mut workspace = self.workspace.clone();
        workspace.add_units([child.clone()]).map_err(|_| {
            Error::new(E_ABORT, crate::ui_text::text("app-workspace-node-capacity"))
        })?;
        workspace
            .connect(crate::workspace::Link {
                id: desktop::document_state::new_unit_id()?,
                from_unit: source.unit.clone(),
                from_port: source.port.clone(),
                to_unit: child,
                to_port: input,
            })
            .map_err(|_| {
                Error::new(
                    E_INVALIDARG,
                    crate::ui_text::text("app-art-input-link-failed"),
                )
            })?;
        self.documents
            .try_reserve(1)
            .map_err(|_| Error::from(E_OUTOFMEMORY))?;
        self.capture_windows
            .borrow_mut()
            .try_reserve(2)
            .map_err(|_| Error::from(E_OUTOFMEMORY))?;
        let document = self.prepare_window_at(
            state,
            visible,
            crate::pin_geometry::PinRect {
                x: rect.right.saturating_add(50).min(i32::MAX - 320),
                y: rect.top.min(i32::MAX - 240),
                width: 320,
                height: 240,
            },
        )?;
        if !guard.matches(self)
            || self.exit_requested.get()
            || self.session_end.paused()
            || self.modals.blocked()
            || self.restore_job.is_some()
            || self.save_job.is_some()
            || !source.current(self)
            || !current(self)
        {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-art-source-preparation-changed"),
            ));
        }
        let hwnd = document.window.0;
        self.workspace = workspace;
        self.capture_windows
            .borrow_mut()
            .extend([hwnd, document.panel.0]);
        self.documents.push(document);
        self.saved_state = None;
        self.display_cache.borrow_mut().take();
        self.propagate_created_child(&source.unit);
        if visible {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
}
