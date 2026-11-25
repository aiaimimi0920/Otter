//! Owned-socket interruption/system-menu/capture-identity checks for Source recovery integration, without adding product fault-injection commands.
use super::*;
#[derive(Default)]
pub(super) struct Progress {
    requested: bool,
    observed: bool,
    resumed: bool,
}
impl Progress {
    pub unsafe fn poll(
        &mut self,
        root: &std::path::Path,
        app: &mut Application,
        identity: &std::sync::Arc<()>,
        hwnd: HWND,
    ) {
        let mode = std::env::var("OTTER_LIVE_SOURCE_RECOVERY_JOINT").unwrap_or_default();
        if mode != "1" && mode != "control" {
            return;
        }
        if !self.requested && root.join("source-disconnect").exists() {
            if mode == "control" {
                // Interrupt only established owned long polls; between requests, wait for the next round instead of canceling TLS handshakes or user applications.
                self.requested = app.remote_sessions.test_disconnect_source_control(identity);
            } else {
                assert!(app.remote_sessions.test_disconnect_source(identity));
                self.requested = true;
            }
        }
        use crate::live_relay_source_runtime::Status;
        let status = app.remote_sessions.source_status(identity);
        let recovering = status == Some(Status::Recovering);
        let phase = if !self.observed && recovering {
            "source-disconnected.json"
        } else if self.observed && !self.resumed && status == Some(Status::Publishing) {
            "source-resumed.json"
        } else {
            return;
        };
        assert!(app.live_sessions[0].is_alive());
        assert!(std::sync::Arc::ptr_eq(
            &app.live_sessions[0].input_identity(),
            identity
        ));
        let mut text = [0u16; 128];
        let count = GetMenuStringW(
            GetSystemMenu(hwnd, false),
            PUBLISH as u32,
            Some(&mut text),
            MF_BYCOMMAND,
        );
        // Network notification may arrive after this Application poll; check the real menu next round, not an old caption that falsely reports an error.
        if String::from_utf16(&text[..count as usize]).unwrap()
            != if recovering {
                crate::ui_text::text("app-live-reconnecting")
            } else {
                crate::ui_text::text("app-live-published")
            }
        {
            return;
        }
        std::fs::write(root.join(phase), serde_json::to_vec(&serde_json::json!({"recovering":recovering,"sameCapture":true,"sameSourceIdentity":true,"menuState":true})).unwrap()).unwrap();
        if recovering {
            self.observed = true;
        } else {
            self.resumed = true;
        }
    }
}
