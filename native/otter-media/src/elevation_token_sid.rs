//! Bounded SID reads from a kernel token, never an account name or profile path.
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_UNEXPECTED, HANDLE},
        Security::{
            GetTokenInformation, TokenIntegrityLevel, TokenUser, TOKEN_INFORMATION_CLASS,
            TOKEN_MANDATORY_LABEL, TOKEN_USER,
        },
    },
};

fn invalid() -> Error {
    Error::new(E_UNEXPECTED, "Invalid token user SID")
}

pub(crate) fn inspect<T>(
    token: HANDLE,
    class: TOKEN_INFORMATION_CLASS,
    read: impl FnOnce(&[u8]) -> Option<T>,
) -> Result<T> {
    let mut data = [0usize; 32];
    let size = std::mem::size_of_val(&data);
    let mut used = 0;
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(data.as_mut_ptr().cast()),
            size as u32,
            &mut used,
        )?;
        if !(std::mem::size_of::<TOKEN_USER>()..=size).contains(&(used as usize)) {
            return Err(invalid());
        }
        let sid = match class {
            c if c == TokenUser => (*data.as_ptr().cast::<TOKEN_USER>()).User.Sid,
            c if c == TokenIntegrityLevel => {
                (*data.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()).Label.Sid
            }
            _ => return Err(invalid()),
        };
        let offset = (sid.0 as usize)
            .checked_sub(data.as_ptr() as usize)
            .ok_or_else(invalid)?;
        let bytes = std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), used as usize);
        let tail = bytes.get(offset..).ok_or_else(invalid)?;
        let n = *tail.get(1).ok_or_else(invalid)?;
        if tail[0] != 1 || !(1..=15).contains(&n) {
            return Err(invalid());
        }
        read(tail.get(..8 + usize::from(n) * 4).ok_or_else(invalid)?).ok_or_else(invalid)
    }
}
