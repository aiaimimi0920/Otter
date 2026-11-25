//! Live and ordinary documents share catalog windows and child-Art commit; no GPU readback, networking, or source freezing during modal intervals.
use super::*;
use desktop::node_menu::art_picker::{dialog, model};
use source::Source;
use std::rc::Rc;
pub(in crate::desktop::application) struct Picker {
    catalog: Option<Rc<desktop::capability_job::Loaded>>,
    origin: Option<String>,
}
impl Picker {
    fn capture(app: &Application) -> Self {
        Self {
            catalog: app.catalog.borrow().loaded.clone(),
            origin: app.authorization.borrow().origin.clone(),
        }
    }
    fn current(&self, app: &Application) -> bool {
        self.origin == app.authorization.borrow().origin
            && match (&self.catalog, &app.catalog.borrow().loaded) {
                (Some(old), Some(now)) => Rc::ptr_eq(old, now),
                (None, None) => true,
                _ => false,
            }
    }
    fn route(&self, app: &Application, index: usize) -> Option<Route> {
        model::route(
            self.catalog.as_deref(),
            index,
            &self.origin,
            &app.authorization.borrow(),
        )
    }
    pub(in crate::desktop::application) unsafe fn show(
        &self,
        app: &Application,
        hwnd: HWND,
    ) -> Result<dialog::Outcome> {
        let catalog = app.catalog.borrow();
        let (loading, failed) = (catalog.job.is_busy() || catalog.requested, catalog.failed);
        drop(catalog);
        let mut affinity = 0;
        GetWindowDisplayAffinity(hwnd, &mut affinity)?;
        dialog::show_with(
            hwnd,
            dialog::Context {
                catalog: self.catalog.clone(),
                origin: self.origin.clone(),
                allowed: &|index| self.route(app, index).is_some(),
                session_end: &app.session_end,
                protected: affinity != WDA_NONE.0,
                loading,
                failed,
            },
        )
    }
}
impl Application {
    pub(in crate::desktop::application) unsafe fn process_live_art_requests(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        let units: Vec<_> = self
            .live_sessions
            .iter()
            .filter(|session| session.take_art_request())
            .map(|session| session.unit_id().clone())
            .collect();
        for unit in units {
            if let Err(error) =
                self.create_live_art_with(&unit, |app, hwnd, picker| picker.show(app, hwnd))
            {
                if !visible {
                    return Err(error);
                }
                // Notify only while the original Live still exists; preserve the source on failure without opening an ownerless blocking window.
                if let Some(owner) = self.live_link_owner(&unit) {
                    MessageBoxW(
                        Some(owner),
                        &HSTRING::from(crate::ui_text::format(
                            "app-live-art-not-added",
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
    pub(in crate::desktop::application) unsafe fn create_live_art_with(
        &mut self,
        unit: &crate::workspace_id::UnitId,
        choose: impl FnOnce(&mut Self, HWND, &Picker) -> Result<dialog::Outcome>,
    ) -> Result<bool> {
        if self.modals.blocked() || self.session_end.paused() || self.live_link_busy() {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-workspace-transition-busy"),
            ));
        }
        let source = Source::capture_live(self, unit)?;
        let guard = restore_batch::guard::Guard::capture(self)?;
        let picker = Picker::capture(self);
        let flag = Rc::new(std::cell::Cell::new(false));
        self.modals.register(&flag);
        let modal = desktop::file_dialog::Guard::begin(&flag).unwrap();
        let outcome = choose(self, source.hwnd, &picker);
        drop(modal);
        let outcome = outcome?;
        if matches!(outcome, dialog::Outcome::Cancel) {
            return Ok(false);
        }
        if !guard.matches(self)
            || !source.current(self)
            || !picker.current(self)
            || self.session_end.paused()
            || self.live_link_busy()
        {
            return Err(Error::new(
                E_ABORT,
                crate::ui_text::text("app-live-art-selection-changed"),
            ));
        }
        let dialog::Outcome::Select(index) = outcome else {
            desktop::capability_catalog::request_for(source.hwnd, &self.catalog)?;
            return Ok(false);
        };
        let route = picker.route(self, index).ok_or_else(|| {
            Error::new(
                E_ABORT,
                crate::ui_text::text("app-art-selection-unavailable"),
            )
        })?;
        let catalog = picker.catalog.as_ref().unwrap();
        let art = catalog
            .handshake
            .capability(index)
            .ok_or_else(|| Error::from(E_INVALIDARG))?;
        let current = |app: &Self| picker.current(app) && picker.route(app, index).is_some();
        match route {
            Route::Backend => self.create_backend_art(&source, catalog.clone(), index, current)?,
            Route::Local(origin) => {
                let config = crate::device_authorization_config::Config::from_env(&origin)
                    .map_err(|_| {
                        Error::new(
                            E_INVALIDARG,
                            crate::ui_text::text("app-art-service-invalid"),
                        )
                    })?;
                self.open_local_art_child(&origin, art.id.into(), unit.as_str(), config, current)?;
            }
        }
        Ok(true)
    }
}
