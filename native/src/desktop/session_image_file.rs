//! 已准入会话文件句柄的后台解码；不解析路径，不隐式访问会话中的网络地址。
use super::{image::Image, Apartment};
use std::{
    fs::File,
    io::{self, Read},
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_FAIL, E_INVALIDARG, E_OUTOFMEMORY},
};
const MAX_BYTES: usize = 64 * 1024 * 1024;

/// 调用者先完成路径/资源权限检查并打开句柄；本函数不把路径字符串视作授权。
/// 应在后台线程调用。文件读取/WIC 系统调用不能强制中断，取消在相邻操作边界生效。
pub fn load(mut file: File, cancel: &AtomicBool) -> Result<Image> {
    check(cancel)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err(Error::new(
            E_INVALIDARG,
            "Session image is not a regular file",
        ));
    }
    let length = usize::try_from(metadata.len())
        .map_err(|_| Error::new(E_INVALIDARG, "Session image exceeds file budget"))?;
    let bytes = read(&mut file, length, cancel)?;
    drop(file); // 完成读取后即释放句柄；WIC 从拥有的内存解码，不重新打开路径。
    check(cancel)?;
    let _apartment = Apartment::new()?;
    let image = Image::from_encoded(&bytes)?;
    check(cancel)?;
    Ok(image)
}
fn io_error(error: io::Error) -> Error {
    Error::new(E_FAIL, format!("Session image read failed: {error}"))
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Session image cancelled"))
    } else {
        Ok(())
    }
}
fn read(
    input: &mut impl Read,
    length: usize,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    check(cancel)?;
    if length == 0 || length > MAX_BYTES {
        return Err(Error::new(
            E_INVALIDARG,
            "Session image exceeds file budget",
        ));
    }
    let mut lease = crate::pixels::reserve(length)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Session encoded image budget exhausted"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Session image allocation failed"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Session image allocation exceeds budget",
        ));
    }
    bytes.resize(length, 0);
    let mut offset = 0;
    while offset < length {
        check(cancel)?;
        let end = (offset + 64 * 1024).min(length);
        let count = match input.read(&mut bytes[offset..end]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        };
        if count == 0 {
            return Err(Error::new(
                E_FAIL,
                "Session image was truncated during reading",
            ));
        }
        offset += count;
    }
    loop {
        check(cancel)?;
        match input.read(&mut [0u8; 1]) {
            Ok(0) => break,
            Ok(_) => return Err(Error::new(E_FAIL, "Session image grew during reading")),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    check(cancel)?;
    Ok(crate::pixels::Pixels::from_reserved(bytes, lease))
}

#[cfg(test)]
#[path = "session_image_file_tests.rs"]
mod tests;
