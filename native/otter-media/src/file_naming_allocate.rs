//! Atomic same-name output allocation creates one new file, without directories/overwriting/completed-file lifetime ownership.
use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
const ATTEMPTS: u32 = 9999;

/// Preserve explicit filenames and return conflicts directly; failure cannot imply overwrite permission.
pub fn create_exact(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Match mainline: the first name has no suffix, followed by _2 through _9999, with each filename stem, including its numeric suffix, limited to 120 characters (excluding the extension).
/// Success transfers ownership to callers; cancellation applies only before allocation, never deleting/revoking successful files.
pub fn create_unique_file(
    directory: &Path,
    stem: &str,
    extension: Option<&str>,
    cancel: &AtomicBool,
) -> io::Result<(File, PathBuf)> {
    allocate_with(directory, stem, extension, cancel, create_exact)
}

pub fn allocate_with<T>(
    directory: &Path,
    stem: &str,
    extension: Option<&str>,
    cancel: &AtomicBool,
    mut create: impl FnMut(&Path) -> io::Result<T>,
) -> io::Result<(T, PathBuf)> {
    if stem.len() > super::MAX_CONTEXT_BYTES
        || extension.is_some_and(|value| value.len() > super::MAX_CONTEXT_BYTES)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Filename input exceeds limit",
        ));
    }
    let mut clean = super::sanitize::Stem::default();
    clean
        .push(stem)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename stem"))?;
    let stem = clean.finish();
    let mut extension: String = extension
        .unwrap_or("png")
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    if extension.is_empty() {
        extension = "png".into();
    }
    for index in 1..=ATTEMPTS {
        if cancel.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Filename allocation cancelled",
            ));
        }
        let suffix = if index == 1 {
            String::new()
        } else {
            format!("_{index}")
        };
        let prefix: String = stem.chars().take(120 - suffix.len()).collect();
        let path = directory.join(format!("{prefix}{suffix}.{extension}"));
        match create(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Unique filename attempts exhausted",
    ))
}

#[cfg(test)]
#[path = "file_naming_allocate_tests.rs"]
mod tests;
