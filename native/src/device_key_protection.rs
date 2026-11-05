//! 与主干 DPAPI1/schema 2 兼容的 Windows 用户级密钥保护；不弹 UI、不降级明文。
use base64::{engine::general_purpose::STANDARD, Engine};
use windows::{
    core::w,
    Win32::{
        Foundation::{LocalFree, HLOCAL},
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    },
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Envelope,
    Protect,
    Unprotect,
}
struct LocalBlob(CRYPT_INTEGER_BLOB);
impl Drop for LocalBlob {
    fn drop(&mut self) {
        if !self.0.pbData.is_null() {
            // OS 分配的输出只由此 guard 释放；明文错误路径也清零。
            unsafe {
                std::slice::from_raw_parts_mut(self.0.pbData, self.0.cbData as usize).zeroize();
                let _ = LocalFree(Some(HLOCAL(self.0.pbData.cast())));
            }
        }
    }
}
pub fn protect(seed: &[u8; 32]) -> Result<String, Error> {
    let mut input = Zeroizing::new(*seed);
    let blob = CRYPT_INTEGER_BLOB {
        cbData: 32,
        pbData: input.as_mut_ptr(),
    };
    let mut output = LocalBlob(CRYPT_INTEGER_BLOB::default());
    unsafe {
        CryptProtectData(
            &blob,
            w!("Otter device identity"),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output.0,
        )
    }
    .map_err(|_| Error::Protect)?;
    if output.0.pbData.is_null() || output.0.cbData == 0 || output.0.cbData > 48 * 1024 {
        return Err(Error::Protect);
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.0.pbData, output.0.cbData as usize) };
    Ok(format!("DPAPI1:{}", STANDARD.encode(bytes)))
}
pub fn unprotect(stored: &str) -> Result<Zeroizing<[u8; 32]>, Error> {
    if stored.len() > 64 * 1024 {
        return Err(Error::Envelope);
    }
    let payload = stored.strip_prefix("DPAPI1:").ok_or(Error::Envelope)?;
    let mut bytes = STANDARD.decode(payload).map_err(|_| Error::Envelope)?;
    let blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_mut_ptr(),
    };
    let mut output = LocalBlob(CRYPT_INTEGER_BLOB::default());
    unsafe {
        CryptUnprotectData(
            &blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output.0,
        )
    }
    .map_err(|_| Error::Unprotect)?;
    if output.0.pbData.is_null() || output.0.cbData != 32 {
        return Err(Error::Unprotect);
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    seed.copy_from_slice(unsafe { std::slice::from_raw_parts(output.0.pbData, 32) });
    Ok(seed)
}
