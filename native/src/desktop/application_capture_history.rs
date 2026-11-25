//! Capture-history application wiring; window/file/reuse operations complete only in the main message loop, without holding RefCell borrows across modal intervals.
use super::*;
use crate::desktop::capture_history_ui::{Action, Origin};
use windows::Win32::Graphics::Gdi::InvalidateRect;
#[path = "capture_history_window.rs"]
pub(super) mod window;
#[path = "capture_history_worker.rs"]
pub(super) mod worker;
impl Application {
    pub(in crate::desktop) fn install_capture_history(&mut self) -> Result<()> {
        let selection = crate::settings_profile::select(
            None,
            std::env::var_os("OTTER_NATIVE_PROFILE_DIR").as_deref(),
            std::env::var_os("LOCALAPPDATA").as_deref(),
            false,
        )
        .map_err(|error| Error::new(E_INVALIDARG, error))?;
        self.capture_history.install(
            selection.and_then(|selection| selection.path.parent().map(|p| p.to_path_buf())),
        );
        Ok(())
    }
    unsafe fn history_source(&self, origin: &Origin) -> Option<&DocumentWindow> {
        self.documents.iter().find(|document| {
            document.is_alive()
                && document.state.unit_id == origin.unit
                && std::rc::Rc::ptr_eq(&document.state.file_dialog, &origin.lifetime)
        })
    }
    pub(super) unsafe fn process_capture_history(&mut self, visible: bool) {
        self.capture_history.poll();
        if self.history_window.as_ref().is_some_and(|window| {
            window
                .handle()
                .is_none_or(|hwnd| !IsWindowVisible(hwnd).as_bool())
        }) {
            self.history_window.take();
        }
        let toggle = self.capture_history.view.borrow_mut().toggle.take();
        if let Some(origin) = toggle {
            if self.history_window.take().is_none() && !self.session_end.paused() {
                if let Some(document) = self.history_source(&origin) {
                    if !document.state.modals.blocked() {
                        match window::Window::new(
                            document.window.0,
                            origin,
                            self.capture_history.view.clone(),
                        ) {
                            Ok(window) => {
                                if visible {
                                    window.show();
                                }
                                self.history_window = Some(window);
                            }
                            Err(error) => {
                                self.capture_history.view.borrow_mut().status =
                                    Some(crate::ui_text::format(
                                        "app-history-open-failed",
                                        &[("error", error.to_string().into())],
                                    ))
                            }
                        }
                    }
                }
            }
        }
        let action = self.capture_history.view.borrow_mut().action.take();
        if let Some((origin, action)) = action {
            if let Err(error) = self.apply_history(&origin, action, visible) {
                self.capture_history.view.borrow_mut().status = Some(crate::ui_text::format(
                    "app-history-operation-incomplete",
                    &[("error", error.to_string().into())],
                ));
            }
        }
        if let Some(window) = &mut self.history_window {
            if let Err(error) = window.update() {
                self.capture_history.view.borrow_mut().status = Some(crate::ui_text::format(
                    "app-history-view-not-updated",
                    &[("error", error.to_string().into())],
                ));
                self.history_window.take();
            }
        }
    }
    unsafe fn apply_history(
        &mut self,
        origin: &Origin,
        action: Action,
        visible: bool,
    ) -> Result<()> {
        self.apply_history_with(
            origin,
            action,
            visible,
            |hwnd, hex, current| crate::desktop::clipboard::copy_text_checked(hwnd, hex, current),
            |hwnd| {
                MessageBoxW(
                    Some(hwnd),
                    &windows::core::HSTRING::from(crate::ui_text::text(
                        "app-history-delete-confirmation",
                    )),
                    w!("Otter"),
                    MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
                ) == IDYES
            },
        )
    }
    unsafe fn apply_history_with(
        &mut self,
        origin: &Origin,
        action: Action,
        visible: bool,
        copy: impl FnOnce(HWND, &str, &dyn Fn() -> bool) -> Result<()>,
        confirm: impl FnOnce(HWND) -> bool,
    ) -> Result<()> {
        let Some(document) = self.history_source(origin) else {
            return Ok(());
        };
        if document.state.modals.blocked() || self.session_end.paused() {
            return Ok(());
        }
        let hwnd = document.window.0;
        match action {
            Action::Color(color, hex) => {
                if document.state.group_locked.get() {
                    return Ok(());
                }
                {
                    let mut canvas = document.state.canvas.borrow_mut();
                    let style = canvas.editor.style().with_color(color);
                    canvas.editor.set_style(style);
                }
                // Copy failure does not undo a successful current-color update; inject no input into user applications.
                let _ = InvalidateRect(Some(hwnd), None, false);
                copy(hwnd, &hex, &|| self.history_source(origin).is_some())?;
            }
            Action::Reuse(id) => {
                let image = self
                    .capture_history
                    .view
                    .borrow()
                    .data
                    .screenshots
                    .iter()
                    .find(|entry| entry.id == id)
                    .and_then(|entry| entry.preview.as_ref())
                    .map(|preview| preview.image.clone());
                if let Some(image) = image {
                    let mut prepared =
                        crate::desktop::open_image::PreparedDocument::from_image(image, true)?;
                    // The original version reused at canvas center; the native version places new pins at the initiating document's current client-area center.
                    let mut client = RECT::default();
                    GetClientRect(hwnd, &mut client)?;
                    let mut center = POINT {
                        x: client.right / 2,
                        y: client.bottom / 2,
                    };
                    windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut center).ok()?;
                    prepared.capture_origin = Some(POINT {
                        x: center.x.saturating_sub(prepared.image.width as i32 / 2),
                        y: center.y.saturating_sub(prepared.image.height as i32 / 2),
                    });
                    self.add_prepared(vec![prepared], visible)?;
                } else {
                    return Err(Error::new(
                        E_FAIL,
                        crate::ui_text::text("app-history-preview-unavailable"),
                    ));
                }
            }
            Action::Delete(id) => {
                let expected = self
                    .capture_history
                    .view
                    .borrow()
                    .data
                    .screenshots
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| (entry.at, entry.thumbnail.clone()));
                let Some(expected) = expected else {
                    return Ok(());
                };
                let Some(_guard) =
                    crate::desktop::file_dialog::Guard::begin(&document.state.file_dialog)
                else {
                    return Ok(());
                };
                let parent = self
                    .history_window
                    .as_ref()
                    .and_then(|window| window.handle())
                    .unwrap_or(hwnd);
                if !confirm(parent) {
                    return Ok(());
                }
                if self.history_source(origin).is_some() && !self.session_end.paused() {
                    let mut view = self.capture_history.view.borrow_mut();
                    let current = view.data.screenshots.iter().any(|entry| {
                        entry.id == id
                            && (entry.at, entry.thumbnail.as_str())
                                == (expected.0, expected.1.as_str())
                    });
                    if current {
                        view.remove_screenshot(&id);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "application_capture_history_tests.rs"]
mod tests;
