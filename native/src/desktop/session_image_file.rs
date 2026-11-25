//! Background decoding of admitted session-file handles; no path parsing or implicit access to session network addresses.
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

/// Callers validate path/resource permissions and open handles first; this function does not treat path strings as authorization.
/// Call on a background thread. File-read/WIC system calls are uninterruptible; cancellation takes effect at adjacent operation boundaries.
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
    drop(file); // Release the handle immediately after reading; WIC decodes owned memory without reopening the path.
    check(cancel)?;
    let _apartment = Apartment::new()?;
    let image = Image::from_encoded_with_cancel(&bytes, &|| cancel.load(Ordering::Acquire));
    check(cancel)?;
    image
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_image_file_tests.rs"]
mod tests;
