//! Parameter windows are bounded document owners; hiding/tray/closure destroys drafts, and actions commit serially in existing UI dispatch segments.
use super::*;
use crate::desktop::document_window::parameter_panel::{self, Panel};
const MAX_OPEN: usize = 4;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_parameter_panel_tests.rs"]
mod tests;
impl Application {
    pub(super) unsafe fn process_parameter_panels(&mut self, visible: bool) {
        let mut open = self
            .documents
            .iter()
            .filter(|doc| doc.parameters.is_some())
            .count();
        let mut changed = false;
        for doc in &mut self.documents {
            let close = doc.state.parameters_ui.close.replace(false)
                || doc.state.group_hidden.get()
                || !doc.is_alive()
                || doc.parameters.as_ref().is_some_and(|panel| panel.retired());
            let toggle = doc.state.parameters_ui.toggle.replace(false);
            if doc.state.local_art_id.is_none() {
                if close {
                    doc.state.parameters_ui.expanded.set(false);
                    if doc.parameters.take().is_some() {
                        open -= 1;
                        changed = true;
                    }
                } else if toggle && parameter_panel::available(&doc.state) {
                    doc.state
                        .parameters_ui
                        .expanded
                        .set(!doc.state.parameters_ui.expanded.get());
                    chrome::drawer_tab(
                        &doc.state.chrome,
                        doc.window.0,
                        doc.state.parameters_ui.expanded.get(),
                    );
                    changed = true;
                }
                if let Some(panel) = &doc.parameters {
                    if let Err(error) = panel.update(&doc.state) {
                        eprintln!("native-parameter-panel-update: {error}");
                        doc.state.parameters_ui.expanded.set(false);
                        doc.parameters.take();
                        open -= 1;
                        changed = true;
                    }
                }
                continue;
            }
            if close || toggle && doc.parameters.is_some() {
                doc.state.parameters_ui.expanded.set(false);
                chrome::drawer_tab(&doc.state.chrome, doc.window.0, false);
                if doc.parameters.take().is_some() {
                    open -= 1;
                    changed = true;
                }
                continue;
            }
            if toggle && parameter_panel::available(&doc.state) {
                let result = if open >= MAX_OPEN {
                    Err(Error::new(
                        E_OUTOFMEMORY,
                        crate::ui_text::text("app-parameter-panel-limit"),
                    ))
                } else {
                    Panel::new(doc.window.0, &doc.state)
                };
                match result {
                    Ok(panel) => {
                        if visible {
                            panel.show();
                        }
                        doc.parameters = Some(panel);
                        chrome::drawer_tab(&doc.state.chrome, doc.window.0, true);
                        open += 1;
                        changed = true;
                    }
                    Err(error) => {
                        eprintln!("native-parameter-panel: {error}");
                        if visible && doc.is_alive() {
                            MessageBoxW(
                                Some(doc.window.0),
                                &HSTRING::from(crate::ui_text::format(
                                    "app-parameter-panel-open-failed",
                                    &[("error", error.to_string().into())],
                                )),
                                w!("Otter"),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                }
            }
            if let Some(panel) = &doc.parameters {
                if let Err(error) = panel.update(&doc.state) {
                    eprintln!("native-parameter-panel-update: {error}");
                    doc.parameters.take();
                    open -= 1;
                    changed = true;
                }
            }
        }
        if changed {
            self.sync_capture_windows();
        }
    }
}
