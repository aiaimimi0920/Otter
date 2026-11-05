//! 同名输出原子分配；只创建一个新文件，不创建目录、不覆盖、不负责完成后的文件寿命。
use std::{
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
const ATTEMPTS: u32 = 9999;

/// 显式文件名保持原样，冲突直接返回；调用方不能把失败解释成可覆盖授权。
pub fn create_exact(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// 与主干一致：首个名称不带后缀，之后为 _2 到 _9999，总 stem 最多 120 字符。
/// 成功时所有权交给调用方；取消只在分配前生效，成功后不再删除或撤销文件。
pub fn create_unique_file(
    directory: &Path,
    stem: &str,
    extension: Option<&str>,
    cancel: &AtomicBool,
) -> io::Result<(File, PathBuf)> {
    allocate_with(directory, stem, extension, cancel, create_exact)
}

pub(crate) fn allocate_with<T>(
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
