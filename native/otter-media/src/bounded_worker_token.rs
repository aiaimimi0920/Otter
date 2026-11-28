//! Compute workers do not inherit administrator or UIAccess authority. The OS
//! linked token is the only downgrade source; never look up shell PIDs,
//! manipulate a login session, prompt for credentials, or retry elevated.
use crate::elevation_status::{self, account};
use std::{
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows::Win32::{Foundation::*, Security::*, System::Threading::*};

// Fixed labels expose the rejection boundary without formatting token contents,
// account identities, session IDs, or other process-specific values.
#[derive(Debug)]
enum Stage {
    TokenQuery,
    ParentElevationType,
    LinkedToken,
    LinkedElevationType,
    LinkedPermissions,
    LinkedIdentity,
    ChildPermissions,
    ChildIdentity,
    WorkerPermissions,
}
#[derive(Debug)]
enum Category {
    ResponseShape,
    Elevated,
    IntegrityAboveMedium,
    UiAccess,
    ElevationType,
    LinkedUnavailable,
    IdentityMismatch,
}
fn denied(stage: Stage, category: Category) -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "Compute worker requires a non-elevated token in the same user session \
             (stage={stage:?}, category={category:?})"
        ),
    )
}
fn token(process: HANDLE) -> io::Result<OwnedHandle> {
    let mut out = HANDLE::default();
    unsafe {
        OpenProcessToken(process, TOKEN_QUERY, &mut out)?;
        Ok(OwnedHandle::from_raw_handle(out.0))
    }
}
// Only called with fixed-size native structs matching their information class.
unsafe fn fixed<T: Default>(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> io::Result<T> {
    let mut value = T::default();
    let mut used = 0;
    GetTokenInformation(
        token,
        class,
        Some((&mut value as *mut T).cast()),
        size_of::<T>() as u32,
        &mut used,
    )?;
    if used as usize != size_of::<T>() {
        return Err(denied(Stage::TokenQuery, Category::ResponseShape));
    }
    Ok(value)
}
#[derive(PartialEq, Eq)]
pub(super) struct Account {
    sid: Vec<u8>,
    session: u32,
}
impl Account {
    fn read(h: HANDLE) -> io::Result<Self> {
        Ok(Self {
            sid: account::inspect(h, TokenUser, |s| Some(s.to_vec()))?,
            session: unsafe { fixed(h, TokenSessionId)? },
        })
    }
}
fn non_ordinary(h: HANDLE) -> io::Result<Option<Category>> {
    let il = account::inspect(h, TokenIntegrityLevel, |s| {
        // S-1-16-RID, with exactly one subauthority.
        (s.len() == 12 && s[..8] == [1, 1, 0, 0, 0, 0, 0, 16])
            .then(|| u32::from_le_bytes(s[8..].try_into().unwrap()))
    })?;
    if elevation_status::query(h)? {
        return Ok(Some(Category::Elevated));
    }
    if il > 0x2000 {
        return Ok(Some(Category::IntegrityAboveMedium));
    }
    if unsafe { fixed::<u32>(h, TokenUIAccess)? } != 0 {
        return Ok(Some(Category::UiAccess));
    }
    Ok(None)
}
pub(super) struct Launch {
    account: Account,
    pub(super) filtered: Option<OwnedHandle>,
}
impl Launch {
    pub(super) fn select() -> io::Result<Self> {
        let parent = token(unsafe { GetCurrentProcess() })?;
        let h = HANDLE(parent.as_raw_handle());
        let account = Account::read(h)?;
        let filtered = if let Some(category) = non_ordinary(h)? {
            if unsafe { fixed::<TOKEN_ELEVATION_TYPE>(h, TokenElevationType)? }
                != TokenElevationTypeFull
            {
                return Err(denied(Stage::ParentElevationType, category));
            }
            let linked = unsafe { fixed::<TOKEN_LINKED_TOKEN>(h, TokenLinkedToken)? }.LinkedToken;
            if linked.is_invalid() {
                return Err(denied(Stage::LinkedToken, Category::LinkedUnavailable));
            }
            let _linked = unsafe { OwnedHandle::from_raw_handle(linked.0) };
            if unsafe { fixed::<TOKEN_ELEVATION_TYPE>(linked, TokenElevationType)? }
                != TokenElevationTypeLimited
            {
                return Err(denied(Stage::LinkedElevationType, Category::ElevationType));
            }
            if let Some(category) = non_ordinary(linked)? {
                return Err(denied(Stage::LinkedPermissions, category));
            }
            if Account::read(linked)? != account {
                return Err(denied(Stage::LinkedIdentity, Category::IdentityMismatch));
            }
            let mut primary = HANDLE::default();
            unsafe {
                DuplicateTokenEx(
                    linked,
                    TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE | TOKEN_QUERY,
                    None,
                    SecurityImpersonation,
                    TokenPrimary,
                    &mut primary,
                )?;
                Some(OwnedHandle::from_raw_handle(primary.0))
            }
        } else {
            None
        };
        Ok(Self { account, filtered })
    }
    pub(super) fn verify_child(&self, process: HANDLE) -> io::Result<()> {
        let child = token(process)?;
        let handle = HANDLE(child.as_raw_handle());
        if let Some(category) = non_ordinary(handle)? {
            return Err(denied(Stage::ChildPermissions, category));
        }
        if Account::read(handle)? != self.account {
            return Err(denied(Stage::ChildIdentity, Category::IdentityMismatch));
        }
        Ok(())
    }
}
pub fn require_ordinary() -> io::Result<()> {
    let own = token(unsafe { GetCurrentProcess() })?;
    if let Some(category) = non_ordinary(HANDLE(own.as_raw_handle()))? {
        Err(denied(Stage::WorkerPermissions, category))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifies_kernel_account_and_session_instead_of_trusting_the_launch_request() {
        let process = unsafe { GetCurrentProcess() };
        let own = token(process).unwrap();
        let h = HANDLE(own.as_raw_handle());
        let mut launch = Launch {
            account: Account::read(h).unwrap(),
            filtered: None,
        };
        assert_eq!(
            launch.verify_child(process).is_ok(),
            non_ordinary(h).unwrap().is_none()
        );
        launch.account.session ^= 1;
        assert!(launch.verify_child(process).is_err());
        launch.account.session ^= 1;
        launch.account.sid.clear();
        assert!(launch.verify_child(process).is_err());
        assert!(launch.verify_child(HANDLE::default()).is_err());
    }
    #[test]
    fn rejection_diagnostic_contains_only_fixed_stage_and_category() {
        let error = denied(Stage::LinkedIdentity, Category::IdentityMismatch);
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            error.to_string(),
            "Compute worker requires a non-elevated token in the same user session \
             (stage=LinkedIdentity, category=IdentityMismatch)"
        );
    }
}
