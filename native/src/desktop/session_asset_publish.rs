//! 受管资源无覆盖发布；原始编码字节保持不变，返回句柄保护至会话提交结束。
use super::{io_error, Root};
use crate::pixels::Pixels;
use std::{
    fs::File,
    io::Read,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

pub enum Slot {
    Source,
    Preview,
}
impl Slot {
    fn name(&self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Preview => "preview",
        }
    }
}
pub struct Published {
    reference: String,
    // 仅共享读，防止已验证资源在上层提交会话之前被改写/删除。
    _file: File,
}
impl Published {
    pub fn reference(&self) -> &str {
        &self.reference
    }
}
impl Root {
    /// 借用会话内联引用；编码字节计入共享预算，不把路径/网络引用作为 Base64 回退。
    pub fn publish_inline(
        &self,
        sticker_id: &str,
        slot: Slot,
        reference: &str,
        cancel: &AtomicBool,
    ) -> Result<Published> {
        check(cancel)?;
        let (mime, encoded) = reference
            .strip_prefix("data:")
            .and_then(|value| value.split_once(";base64,"))
            .ok_or_else(|| invalid("Archive image is not an inline Base64 resource"))?;
        let bytes = crate::desktop::loom_output_inline::decode_bytes(mime, encoded, cancel)?;
        self.publish_image(sticker_id, slot, &bytes, cancel)
    }
    /// 后台入口；显式 Root 是目录权限，不从会话路径自动创建目录或扩权。
    /// 同主干文件名后缀为 png，但保留 WIC 验证过的原始编码，不声称统一重编码。
    pub fn publish_image(
        &self,
        sticker_id: &str,
        slot: Slot,
        bytes: &Pixels,
        cancel: &AtomicBool,
    ) -> Result<Published> {
        check(cancel)?;
        if crate::workspace_id::UnitId::parse(sticker_id).is_none()
            || bytes.is_empty()
            || bytes.len() > 64 * 1024 * 1024
        {
            return Err(invalid("Invalid archive image identity or size"));
        }
        // 解码只是格式/尺寸验证，像素立即释放；原始编码由调用方预算 owner 保持。
        let _apartment = crate::desktop::Apartment::new()?;
        drop(crate::desktop::image::Image::from_encoded(bytes)?);
        check(cancel)?;
        let name = filename(sticker_id, &slot, bytes, cancel)?;
        let reference = format!("{}\\{}", self.requested.trim_end_matches('\\'), name);
        if let Ok(file) = self.open_image(&reference) {
            return verified(file, reference, bytes, cancel);
        }
        let staged = crate::session_file_write::Staged::prepare_bytes(
            &self.resolved.join(&name),
            bytes.clone(),
            cancel,
        )
        .map_err(io_error)?;
        match staged.commit_new(cancel) {
            Ok(_) => verified(
                self.open_published(&reference, cancel)?,
                reference,
                bytes,
                cancel,
            ),
            Err(error) => {
                // 竞争发布者可能已经创建同名资源，必须核验字节，绝不覆盖/删除目标。
                check(cancel)?;
                match self.open_published(&reference, cancel) {
                    Ok(file) => verified(file, reference, bytes, cancel),
                    Err(open) => Err(Error::new(
                        open.code(),
                        format!(
                            "Archive publication commit failed: {error}; reopen failed: {open}"
                        ),
                    )),
                }
            }
        }
    }
    fn open_published(&self, reference: &str, cancel: &AtomicBool) -> Result<File> {
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            check(cancel)?;
            match self.open_image(reference) {
                // 改名已可见时，另一发布者的 DELETE 句柄可能尚未关闭。只在后台有界等待，
                // 不放宽只读 pin，不重写/删除目标；返回后仍须逐字节核验。
                Err(error)
                    if error.code()
                        == windows::core::HRESULT::from_win32(
                            windows::Win32::Foundation::ERROR_SHARING_VIOLATION.0,
                        )
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                result => return result,
            }
        }
    }
}
fn verified(
    mut file: File,
    reference: String,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> Result<Published> {
    check(cancel)?;
    if file.metadata().map_err(io_error)?.len() != bytes.len() as u64 {
        return Err(invalid("Archive filename collision or damaged resource"));
    }
    let mut buffer = [0u8; 64 * 1024];
    for expected in bytes.chunks(buffer.len()) {
        check(cancel)?;
        file.read_exact(&mut buffer[..expected.len()])
            .map_err(io_error)?;
        if &buffer[..expected.len()] != expected {
            return Err(invalid("Archive filename collision or damaged resource"));
        }
    }
    check(cancel)?;
    Ok(Published {
        reference,
        _file: file,
    })
}
fn filename(sticker_id: &str, slot: &Slot, bytes: &[u8], cancel: &AtomicBool) -> Result<String> {
    let sanitized: String = sticker_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let stem = sanitized.trim_matches('_');
    let stem: String = if stem.is_empty() {
        "hook".into()
    } else {
        stem.chars().take(48).collect()
    };
    let mut hash = 0xcbf29ce484222325u64;
    for chunk in bytes.chunks(64 * 1024) {
        check(cancel)?;
        for byte in chunk {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{stem}_{}_{hash:016x}.png", slot.name()))
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Archive image publication cancelled"))
    } else {
        Ok(())
    }
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}

#[cfg(test)]
#[path = "session_asset_publish_tests.rs"]
mod tests;
