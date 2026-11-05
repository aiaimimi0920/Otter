//! 文件仅在用户选择后由唯一文件worker读取；内联恢复不把文件名或URL当读取权限。
use super::super::{image::Image, Apartment};
use crate::sticker_image_params::{image_parts, MAX_FILENAME_BYTES, MAX_FILE_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

pub(in crate::desktop) struct Asset {
    pub uri: String,
    pub filename: String,
    pub image: Image,
    _uri_lease: crate::byte_budget::Lease,
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "图片导入已取消"))
    } else {
        Ok(())
    }
}
fn invalid(message: impl AsRef<str>) -> Error {
    Error::new(E_INVALIDARG, message.as_ref())
}
fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"BM") {
        Some("image/bmp")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}
impl Asset {
    pub fn load(path: &Path, cancel: &AtomicBool) -> Result<Arc<Self>> {
        check(cancel)?;
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("图片文件名无效"))?
            .to_owned();
        if filename.len() > MAX_FILENAME_BYTES || filename.chars().any(char::is_control) {
            return Err(invalid("图片文件名过长或无效"));
        }
        let mut file = std::fs::File::open(path).map_err(|e| invalid(e.to_string()))?;
        let metadata = file.metadata().map_err(|e| invalid(e.to_string()))?;
        if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_FILE_BYTES as u64 {
            return Err(invalid("请选择不超过32 MiB的普通图片文件"));
        }
        let length = metadata.len() as usize;
        let mut raw_lease =
            crate::pixels::reserve(length).ok_or_else(|| invalid("图片读取预算不足"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| invalid("图片读取分配失败"))?;
        if !raw_lease.grow_to(bytes.capacity()) {
            return Err(invalid("图片读取预算不足"));
        }
        bytes.resize(length, 0);
        file.read_exact(&mut bytes)
            .map_err(|e| invalid(e.to_string()))?;
        if file
            .read(&mut [0u8; 1])
            .map_err(|e| invalid(e.to_string()))?
            != 0
        {
            return Err(invalid("图片读取期间发生变化，请重新选择"));
        }
        check(cancel)?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(invalid("图片读取期间超过32 MiB"));
        }
        let mime = mime(&bytes).ok_or_else(|| invalid("仅支持PNG、JPEG、WebP和BMP图片"))?;
        let _apartment = Apartment::new()?;
        let image = Image::from_sticker(&bytes, cancel)?;
        check(cancel)?;
        let prefix = format!("data:{mime};base64,");
        let capacity = prefix.len()
            + base64::encoded_len(bytes.len(), true).ok_or_else(|| invalid("图片编码容量无效"))?;
        let mut uri_lease =
            crate::pixels::reserve(capacity).ok_or_else(|| invalid("图片编码预算不足"))?;
        let mut uri = String::new();
        uri.try_reserve_exact(capacity)
            .map_err(|_| invalid("图片编码分配失败"))?;
        if !uri_lease.grow_to(uri.capacity()) {
            return Err(invalid("图片编码预算不足"));
        }
        uri.push_str(&prefix);
        STANDARD.encode_string(&bytes, &mut uri);
        check(cancel)?;
        Ok(Arc::new(Self {
            uri,
            filename,
            image,
            _uri_lease: uri_lease,
        }))
    }
    pub fn restore(uri: &str, filename: &str, cancel: &AtomicBool) -> Result<Arc<Self>> {
        check(cancel)?;
        let (declared, encoded) = image_parts(uri).map_err(invalid)?;
        // 与Loom共用有界编码缓冲及预算租约，但不扩大它的MIME白名单。
        let bytes = super::super::loom_output_inline::decode_bytes(declared, encoded, cancel)?;
        if bytes.len() > MAX_FILE_BYTES || mime(&bytes) != Some(declared) {
            return Err(invalid("贴图图片格式或容量不符"));
        }
        let _apartment = Apartment::new()?;
        let image = Image::from_sticker(&bytes, cancel)?;
        check(cancel)?;
        let capacity = format!("data:{declared};base64,").len() + encoded.len();
        let mut uri_lease =
            crate::pixels::reserve(capacity).ok_or_else(|| invalid("图片编码预算不足"))?;
        let normalized = format!("data:{declared};base64,{encoded}");
        if !uri_lease.grow_to(normalized.capacity()) {
            return Err(invalid("图片编码预算不足"));
        }
        Ok(Arc::new(Self {
            uri: normalized,
            filename: filename.into(),
            image,
            _uri_lease: uri_lease,
        }))
    }
}
