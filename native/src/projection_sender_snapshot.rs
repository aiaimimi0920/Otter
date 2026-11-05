//! 发送快照仅完整解码一次；压缩 PNG、Base64 和解码像素全部沿原共享预算。
use super::{check, Error};
use crate::{
    loom_base64::Encoded,
    pixels::Pixels,
    projection_delivery::accept::{MAX_PIXELS, MAX_PNG},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

pub(crate) struct Snapshot {
    encoded: Encoded,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Wire<'a> {
    #[serde(borrow)]
    pub(crate) image_base64: &'a str,
    pub(crate) width: u32,
    pub(crate) height: u32,
}
impl Snapshot {
    pub(crate) fn from_png(
        png: &Pixels,
        width: u32,
        height: u32,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        check(cancel)?;
        if png.len() > MAX_PNG
            || !png.starts_with(b"\x89PNG\r\n\x1a\n")
            || width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(Error::Snapshot);
        }
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
            .map_err(|_| Error::Snapshot)?;
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe { CoUninitialize() };
            }
        }
        let _apartment = Apartment;
        let stopped = || cancel.load(Ordering::Acquire);
        let decoded = crate::image_decode::Image::from_projection_png(png, width, height, &stopped)
            .map_err(|_| {
                if stopped() {
                    Error::Stopped
                } else {
                    Error::Snapshot
                }
            })?;
        drop(decoded);
        let mut digest = Sha256::new();
        for chunk in png.chunks(48 * 1024) {
            check(cancel)?;
            digest.update(chunk);
        }
        let encoded = crate::loom_base64::encode(png, cancel).map_err(|_| {
            if stopped() {
                Error::Stopped
            } else {
                Error::Limit
            }
        })?;
        check(cancel)?;
        Ok(Self {
            encoded,
            width,
            height,
            digest: format!("{:x}", digest.finalize()),
        })
    }
    pub(crate) fn decode(wire: Wire<'_>, cancel: &AtomicBool) -> Result<Self, Error> {
        let png = Self::decode_png(&wire, cancel)?;
        Self::from_png(&png, wire.width, wire.height, cancel)
    }
    /// 已验签/已验尺寸的不可变快照仍用同一有界Base64入口，不造第二份codec。
    pub(crate) fn png(&self, cancel: &AtomicBool) -> Result<Pixels, Error> {
        Self::decode_png(&self.wire(), cancel)
    }
    fn decode_png(wire: &Wire<'_>, cancel: &AtomicBool) -> Result<Pixels, Error> {
        check(cancel)?;
        if wire.image_base64.is_empty() || wire.image_base64.len() > MAX_PNG.div_ceil(3) * 4 {
            return Err(Error::Snapshot);
        }
        let capacity = wire.image_base64.len().div_ceil(4) * 3;
        let mut lease = crate::pixels::reserve(capacity).ok_or(Error::Limit)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| Error::Limit)?;
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Limit);
        }
        STANDARD
            .decode_vec(wire.image_base64, &mut bytes)
            .map_err(|_| Error::Snapshot)?;
        check(cancel)?;
        Ok(Pixels::from_reserved(bytes, lease))
    }
    fn wire(&self) -> Wire<'_> {
        Wire {
            image_base64: self.encoded.as_str(),
            width: self.width,
            height: self.height,
        }
    }
}
impl Serialize for Snapshot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.wire().serialize(serializer)
    }
}
