//! 文件参数 Data URL 编码；供后台 worker 使用，不解码图片、不保留路径、不访问网络。
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

/// 仅由文件选择产生的路径调用。取消在每个读取/编码边界检查；系统阻塞读取不能强制中断。
/// 单值编码上限复用参数预算，最终提交仍须核验全部参数及 JSON 包装的总开销。
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
    if output
        .len()
        .checked_add(length)
        .is_none_or(|size| size > limit)
    {
        return Err(Error::Capacity);
    }
    output
        .try_reserve_exact(length)
        .map_err(|_| Error::Capacity)?;
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
    // 只保留一个固定缓冲区和最终字符串；任意短读的余数留到下次，避免中间 base64 填充。
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
    // 与浏览器文件选择一样按扩展名分类，不嗅探/执行内容；未知类型安全回退为二进制。
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
