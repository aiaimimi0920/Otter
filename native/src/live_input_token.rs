//! 有界、对齐的 token 信息读取；只复制 SID，所有内嵌指针须落在返回缓冲区内。
use super::{Denied, Result};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows::Win32::{Foundation::HANDLE, Security::*, System::Threading::OpenProcessToken};
pub(super) struct Identity {
    pub session: u32,
    pub integrity: u32,
    pub sid: Vec<u8>,
}
pub(super) fn read(process: HANDLE) -> Result<Identity> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }
        .map_err(|_| Denied::PermissionDenied)?;
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    let handle = HANDLE(token.as_raw_handle());
    let session = information(handle, TokenSessionId)?;
    let session = unsafe { *session.as_ptr().cast::<u32>() };
    let label = information(handle, TokenIntegrityLevel)?;
    let label_sid = unsafe { (*label.as_ptr().cast::<TOKEN_MANDATORY_LABEL>()).Label.Sid };
    let integrity_sid = sid(&label, label_sid)?;
    if integrity_sid.len() < 12 || integrity_sid[2..8] != [0, 0, 0, 0, 0, 16] {
        return Err(Denied::PermissionDenied);
    }
    let integrity =
        u32::from_le_bytes(integrity_sid[integrity_sid.len() - 4..].try_into().unwrap());
    let user = information(handle, TokenUser)?;
    let user_sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    Ok(Identity {
        session,
        integrity,
        sid: sid(&user, user_sid)?.to_vec(),
    })
}
struct Buffer {
    words: Vec<usize>,
    length: usize,
}
impl Buffer {
    fn as_ptr(&self) -> *const usize {
        self.words.as_ptr()
    }
}
fn information(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> Result<Buffer> {
    let mut needed = 0;
    let _ = unsafe { GetTokenInformation(token, class, None, 0, &mut needed) };
    let minimum = if class == TokenSessionId {
        4
    } else {
        std::mem::size_of::<TOKEN_USER>() as u32
    };
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
    Ok(Buffer {
        words: storage,
        length: needed as usize,
    })
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
        let mut data = Buffer {
            words: vec![0usize; 4],
            length: 4 * std::mem::size_of::<usize>(),
        };
        let pointer = PSID(data.words.as_mut_ptr().cast());
        assert!(sid(&data, PSID::default()).is_err());
        assert!(sid(&data, pointer).is_err());
        data.words[0] = 0x101;
        assert_eq!(sid(&data, pointer).unwrap().len(), 12);
        data.length = 11;
        assert!(sid(&data, pointer).is_err());
        data.length = 4 * std::mem::size_of::<usize>();
        data.words[0] = 0x1001;
        assert!(sid(&data, pointer).is_err());
        data.words[0] = 0x0f01;
        assert!(sid(&data, pointer).is_err());
    }
}
