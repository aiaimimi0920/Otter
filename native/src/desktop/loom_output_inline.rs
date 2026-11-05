//! Loom 内联图片输出转原生 PBGRA；仅后台调用，不信任响应中的尺寸提示。
use super::{image::Image, Apartment};
use base64::Engine;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};
const MAX_ENCODED: usize = 64 * 1024 * 1024;

/// 编码缓冲和解码像素均计入共享预算，WIC 内部工作内存沿用现有解码边界。
pub fn decode(value: &Value, cancel: &AtomicBool) -> Result<Image> {
    check_cancel(cancel)?;
    if value["kind"].as_str() != Some("inline_resource") {
        return Err(Error::new(E_INVALIDARG, "Unsupported Loom inline image"));
    }
    decode_base64(
        value["mime"].as_str().unwrap_or(""),
        value["dataBase64"].as_str().unwrap_or(""),
        cancel,
    )
}
/// 借用编码文本供参数与会话资源共用，不构造另一份 JSON/Base64 副本。
pub(super) fn decode_base64(mime: &str, encoded: &str, cancel: &AtomicBool) -> Result<Image> {
    let bytes = decode_bytes(mime, encoded, cancel)?;
    let _apartment = Apartment::new()?;
    let image = Image::from_encoded(&bytes)?;
    check_cancel(cancel)?;
    Ok(image)
}
/// 共用有界编码字节 owner；调用者仍须经 WIC 验证后才能把内容当图片发布。
pub(super) fn decode_bytes(
    mime: &str,
    encoded: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    check_cancel(cancel)?;
    if !matches!(
        mime,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "image/bmp"
    ) {
        return Err(Error::new(E_INVALIDARG, "Unsupported Loom inline image"));
    }
    if encoded.is_empty()
        || encoded.starts_with("data:")
        || encoded.len() > MAX_ENCODED.div_ceil(3) * 4
    {
        return Err(Error::new(E_INVALIDARG, "Invalid Loom inline data"));
    }
    let capacity = base64::decoded_len_estimate(encoded.len());
    let mut lease = crate::pixels::reserve(capacity)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Loom image budget exhausted"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Loom image allocation failed"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Loom image capacity exceeds budget",
        ));
    }
    bytes.resize(capacity, 0);
    let size = base64::engine::general_purpose::STANDARD
        .decode_slice(encoded, &mut bytes)
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid Loom image Base64"))?;
    if size > MAX_ENCODED {
        return Err(Error::new(
            E_INVALIDARG,
            "Loom image exceeds encoded budget",
        ));
    }
    bytes.truncate(size);
    let bytes = crate::pixels::Pixels::from_reserved(bytes, lease);
    check_cancel(cancel)?;
    Ok(bytes)
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Loom output cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "loom_output_inline_tests.rs"]
mod tests;
