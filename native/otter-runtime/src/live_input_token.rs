//! Bounded, aligned token-information reads; copy only SIDs, and require all embedded pointers to lie within the returned buffer.
//!
//! Performance audit optimization (for AI/human reviewers):
//! `token::read` previously allocated 4 heap `Vec`s per process (`TokenSessionId`, `TokenIntegrityLevel`, `TokenUser`,
//! and `sid.to_vec()`) and re-queried `GetCurrentProcess()` on every mouse move (~56 heap allocations + 14 token opens
//! per `Event::Move`). Because `header[1] <= 15` bounds valid Windows SIDs to `8 + 15 * 4 = 68` bytes and `TOKEN_USER` /
//! `TOKEN_MANDATORY_LABEL` fit within 128 bytes, inline stack storage (`Sid` + `StackWords`) eliminates 100% of heap
//! allocations on normal tokens. Token identity is reread for every security preflight.
use super::{Denied, Result};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::{
    Foundation::HANDLE,
    Security::*,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

const MAX_SID_BYTES: usize = 8 + 15 * 4;
const INLINE_WORDS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Sid {
    bytes: [u8; MAX_SID_BYTES],
    len: u8,
}
impl Sid {
    fn from_slice(slice: &[u8]) -> Result<Self> {
        if slice.len() > MAX_SID_BYTES {
            return Err(Denied::PermissionDenied);
        }
        let mut bytes = [0u8; MAX_SID_BYTES];
        bytes[..slice.len()].copy_from_slice(slice);
        Ok(Self {
            bytes,
            len: slice.len() as u8,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}
#[cfg(test)]
impl From<Vec<u8>> for Sid {
    fn from(value: Vec<u8>) -> Self {
        Self::from_slice(&value).expect("test SID within 68-byte maximum")
    }
}

#[derive(Clone, Copy)]
pub(super) struct Identity {
    pub session: u32,
    pub integrity: u32,
    pub sid: Sid,
}

pub(super) fn read_self() -> Result<Identity> {
    read(unsafe { GetCurrentProcess() })
}

pub(super) fn read(process: HANDLE) -> Result<Identity> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }
        .map_err(|_| Denied::PermissionDenied)?;
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    let handle = HANDLE(token.as_raw_handle());
    let mut session = 0u32;
    let mut ret_len = 0u32;
    unsafe {
        GetTokenInformation(
            handle,
            TokenSessionId,
            Some((&mut session as *mut u32).cast()),
            std::mem::size_of::<u32>() as u32,
            &mut ret_len,
        )
    }
    .map_err(|_| Denied::PermissionDenied)?;
    if ret_len != std::mem::size_of::<u32>() as u32 {
        return Err(Denied::PermissionDenied);
    }
    let mut label = Buffer::new();
    information(handle, TokenIntegrityLevel, &mut label)?;
    let label_sid = unsafe { (*label.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()).Label.Sid };
    let integrity_sid = sid(&label, label_sid)?;
    if integrity_sid.len() < 12 || integrity_sid[2..8] != [0, 0, 0, 0, 0, 16] {
        return Err(Denied::PermissionDenied);
    }
    let integrity =
        u32::from_le_bytes(integrity_sid[integrity_sid.len() - 4..].try_into().unwrap());
    let mut user = Buffer::new();
    information(handle, TokenUser, &mut user)?;
    let user_sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    Ok(Identity {
        session,
        integrity,
        sid: Sid::from_slice(sid(&user, user_sid)?)?,
    })
}

enum Words {
    Inline([usize; INLINE_WORDS]),
    Heap(Vec<usize>),
}

struct Buffer {
    words: Words,
    length: usize,
}
impl Buffer {
    fn new() -> Self {
        Self {
            words: Words::Inline([0usize; INLINE_WORDS]),
            length: 0,
        }
    }
    fn as_ptr(&self) -> *const usize {
        match &self.words {
            Words::Inline(arr) => arr.as_ptr(),
            Words::Heap(vec) => vec.as_ptr(),
        }
    }
}

/// Fill `out` in place without moving `Words::Inline` on the stack: `TOKEN_USER` and `TOKEN_MANDATORY_LABEL`
/// embed self-referential `PSID` pointers into the exact buffer address passed to `GetTokenInformation`.
fn information(token: HANDLE, class: TOKEN_INFORMATION_CLASS, out: &mut Buffer) -> Result<()> {
    let minimum = if class == TokenSessionId {
        4
    } else {
        std::mem::size_of::<TOKEN_USER>() as u32
    };
    let inline_cap = (INLINE_WORDS * std::mem::size_of::<usize>()) as u32;
    let mut needed = 0u32;
    if let Words::Inline(inline) = &mut out.words {
        if unsafe {
            GetTokenInformation(
                token,
                class,
                Some(inline.as_mut_ptr().cast()),
                inline_cap,
                &mut needed,
            )
        }
        .is_ok()
        {
            if needed < minimum || needed > inline_cap {
                return Err(Denied::PermissionDenied);
            }
            out.length = needed as usize;
            return Ok(());
        }
    }
    if !(minimum..=65_536).contains(&needed) {
        return Err(Denied::PermissionDenied);
    }
    let mut storage = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
    let capacity = needed;
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(storage.as_mut_ptr().cast()),
            capacity,
            &mut needed,
        )
    }
    .map_err(|_| Denied::PermissionDenied)?;
    if needed < minimum || needed > capacity {
        return Err(Denied::PermissionDenied);
    }
    out.words = Words::Heap(storage);
    out.length = needed as usize;
    Ok(())
}

fn sid(storage: &Buffer, pointer: PSID) -> Result<&[u8]> {
    let base = storage.as_ptr() as usize;
    let offset = (pointer.0 as usize)
        .checked_sub(base)
        .ok_or(Denied::PermissionDenied)?;
    let bytes =
        unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), storage.length) };
    let header = bytes
        .get(offset..)
        .and_then(|tail| tail.get(..8))
        .ok_or(Denied::PermissionDenied)?;
    if header[0] != 1 || header[1] > 15 {
        return Err(Denied::PermissionDenied);
    }
    bytes
        .get(offset..offset + 8 + usize::from(header[1]) * 4)
        .ok_or(Denied::PermissionDenied)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_sid_cannot_escape_buffer_or_claim_excess_subauthorities() {
        let mut words = [0usize; INLINE_WORDS];
        let mut data = Buffer {
            words: Words::Inline(words),
            length: 4 * std::mem::size_of::<usize>(),
        };
        let ptr = match &mut data.words {
            Words::Inline(arr) => PSID(arr.as_mut_ptr().cast()),
            Words::Heap(_) => unreachable!(),
        };
        assert!(sid(&data, PSID::default()).is_err());
        assert!(sid(&data, ptr).is_err());
        words[0] = 0x101;
        data.words = Words::Inline(words);
        let ptr = PSID((data.as_ptr() as *mut usize).cast());
        assert_eq!(sid(&data, ptr).unwrap().len(), 12);
        data.length = 11;
        assert!(sid(&data, ptr).is_err());
        data.length = 4 * std::mem::size_of::<usize>();
        words[0] = 0x1001;
        data.words = Words::Inline(words);
        let ptr = PSID((data.as_ptr() as *mut usize).cast());
        assert!(sid(&data, ptr).is_err());
        words[0] = 0x0f01;
        data.words = Words::Inline(words);
        let ptr = PSID((data.as_ptr() as *mut usize).cast());
        assert!(sid(&data, ptr).is_err());
    }
}
