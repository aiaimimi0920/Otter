//! File-parameter Data URL encoding for background workers; no image decoding, path retention, or network access.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug)]
pub enum Error {
    Cancelled,
    Capacity,
    NotFile,
    Io(io::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("文件导入已取消"),
            Self::Capacity => formatter.write_str("文件超过参数容量限制"),
            Self::NotFile => formatter.write_str("请选择普通文件"),
            Self::Io(error) => write!(formatter, "文件读取失败：{error}"),
        }
    }
}
impl std::error::Error for Error {}

/// Call only with paths produced by file selection. Check cancellation at every read/encoding boundary; blocking system reads cannot be forcibly interrupted.
/// Single-value encoding reuses the parameter-budget limit; final commit must still check total overhead for all parameters and JSON wrapping.
pub fn load(path: &Path, cancel: &AtomicBool) -> Result<String, Error> {
    check(cancel)?;
    let file = File::open(path).map_err(Error::Io)?;
    let metadata = file.metadata().map_err(Error::Io)?;
    if !metadata.is_file() {
        return Err(Error::NotFile);
    }
    let limit = crate::loom_parameter_state::MAX_BYTES;
    if metadata.len() > (limit / 4 * 3) as u64 {
        return Err(Error::Capacity);
    }
    encode(file, mime(path), cancel, limit)
}

fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

fn append(output: &mut String, bytes: &[u8], limit: usize) -> Result<(), Error> {
    let length = base64::encoded_len(bytes.len(), true).ok_or(Error::Capacity)?;
    let needed = output
        .len()
        .checked_add(length)
        .filter(|&size| size <= limit)
        .ok_or(Error::Capacity)?;
    if needed > output.capacity() {
        // Short reads should not trigger expansion every time; requested growth and actual output both stay within the parameter budget.
        let capacity = needed.max(output.capacity().saturating_mul(2).min(limit));
        output
            .try_reserve_exact(capacity - output.len())
            .map_err(|_| Error::Capacity)?;
    }
    STANDARD.encode_string(bytes, output);
    Ok(())
}

fn encode(
    mut input: impl Read,
    mime: &str,
    cancel: &AtomicBool,
    limit: usize,
) -> Result<String, Error> {
    let mut output = format!("data:{mime};base64,");
    if output.len() > limit {
        return Err(Error::Capacity);
    }
    // Retain only one fixed buffer and the final string; carry arbitrary short-read remainders forward to avoid intermediate base64 padding.
    let mut buffer = [0u8; 12 * 1024];
    let mut retained = 0;
    loop {
        check(cancel)?;
        let count = match input.read(&mut buffer[retained..]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(Error::Io(error)),
        };
        check(cancel)?;
        if count == 0 {
            append(&mut output, &buffer[..retained], limit)?;
            return Ok(output);
        }
        let total = retained + count;
        let complete = total / 3 * 3;
        append(&mut output, &buffer[..complete], limit)?;
        buffer.copy_within(complete..total, 0);
        retained = total - complete;
    }
}

fn mime(path: &Path) -> &'static str {
    // Classify by extension like browser file selection, without sniffing/executing content; unknown types safely fall back to binary.
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
#[path = "parameter_file_tests.rs"]
mod tests;
