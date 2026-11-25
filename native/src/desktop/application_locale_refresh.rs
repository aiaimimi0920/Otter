//! Refresh existing presentation owners without rebuilding documents or replacing editable drafts.
use super::*;
impl Application {
    pub(in crate::desktop::application) unsafe fn refresh_display_language(&mut self) {
        crate::ui_text::set_language(self.display_language_status().language);
        for document in &self.documents {
            if !document.is_alive() {
                continue;
            }
            crate::desktop::tool_panel::refresh_language(&document.state);
            crate::desktop::pin_settings::refresh(&document.state);
            crate::desktop::edit_properties::refresh(&document.state);
            crate::desktop::tray::refresh_language(document.window.0, &document.state);
            if let Err(error) = crate::desktop::extension_overlay::refresh_language(
                document.window.0,
                &document.state,
            ) {
                eprintln!("native-extension-view-language: {error}");
            }
            let _ = windows::Win32::Graphics::Gdi::RedrawWindow(
                Some(document.window.0),
                None,
                None,
                windows::Win32::Graphics::Gdi::RDW_INVALIDATE
                    | windows::Win32::Graphics::Gdi::RDW_ALLCHILDREN,
            );
        }
        self.sync_workspace_chrome(true);
    }
}
