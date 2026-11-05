//! 按已打开句柄核验 owner/DACL；仅 current user、SYSTEM、Administrators 可以修改候选。
use super::invalid;
use std::{
    fs::File,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows::Win32::{
    Foundation::{LocalFree, HANDLE, HLOCAL},
    Security::{
        Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
        *,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}
fn trusted(sid: PSID, user: PSID) -> bool {
    unsafe {
        !sid.0.is_null()
            && !user.0.is_null()
            && IsValidSid(sid).as_bool()
            && (EqualSid(sid, user).is_ok()
                || IsWellKnownSid(sid, WinLocalSystemSid).as_bool()
                || IsWellKnownSid(sid, WinBuiltinAdministratorsSid).as_bool())
    }
}
fn sid_within(bytes: &[u8], pointer: PSID) -> bool {
    let Some(offset) = (pointer.0 as usize).checked_sub(bytes.as_ptr() as usize) else {
        return false;
    };
    let Some(sid) = bytes.get(offset..) else {
        return false;
    };
    sid.len() >= 8 && sid[0] == 1 && sid[1] <= 15 && sid.len() >= 8 + usize::from(sid[1]) * 4
}
pub(super) fn check(file: &File) -> io::Result<()> {
    check_mask(file, 0x500D_0156)
}
pub(super) fn ancestor(file: &File) -> io::Result<()> {
    // 属性/EA、改 ACL/owner 不受目录共享锁保护；其他用户不得拥有这些权限。
    check_mask(file, 0x500C_0110)
}
fn check_mask(file: &File, modify: u32) -> io::Result<()> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }?;
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    // SID 最多 68 字节；OS 返回的 TOKEN_USER 指针只在这个对齐缓冲区寿命内使用。
    let mut storage = [0usize; 128];
    let mut length = 0;
    unsafe {
        GetTokenInformation(
            HANDLE(token.as_raw_handle()),
            TokenUser,
            Some(storage.as_mut_ptr().cast()),
            std::mem::size_of_val(&storage) as u32,
            &mut length,
        )
    }?;
    if length as usize > std::mem::size_of_val(&storage)
        || (length as usize) < std::mem::size_of::<TOKEN_USER>()
    {
        return Err(invalid());
    }
    let user = unsafe { (*storage.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    if !sid_within(
        unsafe { std::slice::from_raw_parts(storage.as_ptr().cast(), length as usize) },
        user,
    ) {
        return Err(invalid());
    }
    let mut owner = PSID::default();
    let mut dacl = std::ptr::null_mut();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        GetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            Some(&mut dacl),
            None,
            Some(&mut descriptor),
        )
    }
    .ok()?;
    let _descriptor = Descriptor(descriptor);
    if descriptor.0.is_null() || dacl.is_null() {
        return Err(invalid());
    }
    let length = unsafe { GetSecurityDescriptorLength(descriptor) } as usize;
    if !(20..=131072).contains(&length) {
        return Err(invalid());
    }
    let security = unsafe { std::slice::from_raw_parts(descriptor.0.cast::<u8>(), length) };
    if !sid_within(security, owner) || !trusted(owner, user) {
        return Err(invalid());
    }
    let offset = (dacl as usize)
        .checked_sub(security.as_ptr() as usize)
        .ok_or_else(invalid)?;
    let tail = security.get(offset..).ok_or_else(invalid)?;
    let header = tail.get(..8).ok_or_else(invalid)?;
    let acl_bytes = tail
        .get(..usize::from(u16::from_le_bytes([header[2], header[3]])))
        .ok_or_else(invalid)?;
    if !unsafe { IsValidAcl(dacl).as_bool() } {
        return Err(invalid());
    }
    let mut info = ACL_SIZE_INFORMATION::default();
    unsafe {
        GetAclInformation(
            dacl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            std::mem::size_of_val(&info) as u32,
            AclSizeInformation,
        )
    }?;
    if info.AceCount > 1024 || info.AclBytesInUse > 65535 {
        return Err(invalid());
    }
    for index in 0..info.AceCount {
        let mut pointer = std::ptr::null_mut();
        unsafe { GetAce(dacl, index, &mut pointer) }?;
        let offset = (pointer as usize)
            .checked_sub(acl_bytes.as_ptr() as usize)
            .ok_or_else(invalid)?;
        let tail = acl_bytes.get(offset..).ok_or_else(invalid)?;
        let header = tail.get(..4).ok_or_else(invalid)?;
        let bytes = tail
            .get(..usize::from(u16::from_le_bytes([header[2], header[3]])))
            .ok_or_else(invalid)?;
        // 标准 deny 不授予权限；callback/object/未知 ACE 不作乐观推断。
        if header[0] == 1 {
            continue;
        }
        if header[0] != 0 || bytes.len() < 16 {
            return Err(invalid());
        }
        let mask = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| invalid())?);
        let sid = PSID(bytes[8..].as_ptr().cast_mut().cast());
        if !sid_within(bytes, sid) {
            return Err(invalid());
        }
        // inherit-only 对当前对象不生效；每个实际子级会独立检查生效的 ACE。
        if header[1] & INHERIT_ONLY_ACE.0 as u8 != 0 {
            continue;
        }
        if mask & modify != 0 && !trusted(sid, user) {
            return Err(invalid());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shader_candidate_sid_checks_null_bounds_revision_and_count_before_ffi() {
        assert!(!trusted(PSID::default(), PSID::default()));
        let mut bytes = [0u8; 16];
        let pointer = PSID(bytes.as_mut_ptr().cast());
        assert!(!sid_within(&bytes, pointer));
        bytes[0] = 1;
        bytes[1] = 2;
        assert!(sid_within(&bytes, pointer));
        assert!(!sid_within(&bytes[..15], pointer));
        bytes[1] = 16;
        assert!(!sid_within(&bytes, pointer));
        assert!(!sid_within(&bytes, PSID::default()));
        assert!(!sid_within(
            &bytes,
            PSID(bytes.as_ptr().wrapping_add(16).cast_mut().cast())
        ));
    }
}
