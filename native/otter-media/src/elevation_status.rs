//! Read the effective elevation of the current process, not its account membership.
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_UNEXPECTED, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};
#[path = "elevation_token_sid.rs"]
pub(crate) mod account;

/// Discovery and device requests from an ordinary process must never drive an
/// elevated desktop. This process token does not change during a desktop run.
pub fn allows_desktop_integrations() -> bool {
    static ORDINARY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ORDINARY.get_or_init(|| crate::bounded_worker_process::require_ordinary_token().is_ok())
}

pub fn elevated() -> Result<bool> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)? };
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    query(HANDLE(token.as_raw_handle()))
}

pub(crate) fn query(token: HANDLE) -> Result<bool> {
    let mut info = TOKEN_ELEVATION::default();
    let size = std::mem::size_of_val(&info) as u32;
    let mut returned = 0;
    unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut info as *mut TOKEN_ELEVATION).cast()),
            size,
            &mut returned,
        )?;
    }
    if returned != size {
        return Err(Error::new(
            E_UNEXPECTED,
            "Incomplete TokenElevation response",
        ));
    }
    Ok(info.TokenIsElevated != 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_actual_token_and_never_reports_an_invalid_handle_as_standard_user() {
        let _value = super::elevated().unwrap();
        assert!(super::query(Default::default()).is_err());
    }
}
