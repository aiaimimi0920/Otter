//! Input-security preflight is not user authorization; recheck on every enable and before every send, without caching success.
use super::Target;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::{Foundation::*, System::Threading::*};
#[path = "live_input_desktop.rs"]
pub mod desktop;
#[path = "live_input_token.rs"]
pub mod token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    SourceChanged,
    SecureDesktop,
    DifferentSession,
    PermissionDenied,
}
pub type Result<T> = std::result::Result<T, Denied>;

/// No privilege elevation, desktop switching, or input sending; success only means this snapshot may proceed to permission checks.
pub fn preflight(target: &Target) -> Result<()> {
    target.validate().map_err(|_| Denied::SourceChanged)?;
    desktop::check()?;
    let process = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            target.identity.process,
        )
    }
    .map_err(|_| Denied::PermissionDenied)?;
    let process = unsafe { OwnedHandle::from_raw_handle(process.0) };
    // The original process synchronization handle remains alive and HWND identity unchanged, preventing target exit/reuse while opening by PID.
    target.validate().map_err(|_| Denied::SourceChanged)?;
    // Avoid redundant inner `desktop::check()` here since lines 22 and 38 already bracket `check_process_tokens`.
    check_process_tokens(HANDLE(process.as_raw_handle()))?;
    target.validate().map_err(|_| Denied::SourceChanged)?;
    desktop::check()
}
/// Use the original process query handle held by the caller; do not rebind another process using a cached PID.
pub fn check_process(process: HANDLE) -> Result<()> {
    desktop::check()?;
    check_process_tokens(process)
}
pub(crate) fn check_process_tokens(process: HANDLE) -> Result<()> {
    let controller = token::read_self()?;
    let source = token::read(process)?;
    allows(&controller, &source)
}
fn allows(controller: &token::Identity, source: &token::Identity) -> Result<()> {
    if controller.session != source.session {
        return Err(Denied::DifferentSession);
    }
    if controller.sid.is_empty()
        || controller.sid != source.sid
        || controller.integrity < source.integrity
    {
        return Err(Denied::PermissionDenied);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the interactive default desktop; owns a hidden fixture and sends no input"]
    fn actual_owned_target_preflight_and_destroyed_target_rejection() {
        use windows::{core::w, Win32::UI::WindowsAndMessaging::*};
        struct Window(HWND);
        impl Drop for Window {
            fn drop(&mut self) {
                unsafe {
                    let _ = DestroyWindow(self.0);
                }
            }
        }
        let window = Window(
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!(""),
                    WS_POPUP,
                    0,
                    0,
                    16,
                    16,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .unwrap(),
        );
        let target = Target::bind(window.0).unwrap();
        assert_eq!(preflight(&target), Ok(()));
        drop(window);
        assert_eq!(preflight(&target), Err(Denied::SourceChanged));
    }
    #[test]
    fn policy_rejects_cross_user_session_and_upward_integrity() {
        let mut controller = token::Identity {
            session: 1,
            integrity: 8192,
            sid: vec![1].into(),
        };
        let mut source = token::Identity {
            session: 1,
            integrity: 8192,
            sid: vec![1].into(),
        };
        assert_eq!(allows(&controller, &source), Ok(()));
        source.integrity += 1;
        assert_eq!(allows(&controller, &source), Err(Denied::PermissionDenied));
        controller.integrity = 16384;
        assert_eq!(allows(&controller, &source), Ok(()));
        source.sid = vec![2].into();
        assert_eq!(allows(&controller, &source), Err(Denied::PermissionDenied));
        source.sid.clear();
        controller.sid.clear();
        assert_eq!(allows(&controller, &source), Err(Denied::PermissionDenied));
        source.session = 2;
        assert_eq!(allows(&controller, &source), Err(Denied::DifferentSession));
    }
    #[test]
    fn actual_process_token_has_stable_identity_without_input_or_elevation() {
        let first = token::read(unsafe { GetCurrentProcess() }).unwrap();
        let second = token::read(unsafe { GetCurrentProcess() }).unwrap();
        assert!(!first.sid.is_empty());
        assert_eq!(allows(&first, &second), Ok(()));
        assert!(token::read(HANDLE::default()).is_err());
    }
}
