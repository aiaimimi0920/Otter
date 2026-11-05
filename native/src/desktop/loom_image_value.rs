//! 正式图像的显示像素与可选原始 PNG 共用不可变 owner；别名和快照只克隆 Arc。
//! PNG 必须来自同次可信编码或同份已解码字节，不能把任意描述符与图片拼装发布。
use super::{export, image::Image};
use crate::pixels::Pixels;
use std::{
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

#[derive(Clone)]
pub struct ImageValue {
    image: Image,
    png: Option<Pixels>,
}
impl From<Image> for ImageValue {
    fn from(image: Image) -> Self {
        Self { image, png: None }
    }
}
impl Deref for ImageValue {
    type Target = Image;
    fn deref(&self) -> &Image {
        &self.image
    }
}
impl ImageValue {
    pub fn image(&self) -> &Image {
        &self.image
    }
    pub fn encoded_png(&self) -> Option<&Pixels> {
        self.png.as_ref()
    }
    pub(crate) fn into_image(self) -> Image {
        self.image
    }
    pub(crate) fn bytes(&self) -> usize {
        self.image
            .pixels
            .len()
            .saturating_add(self.png.as_ref().map_or(0, |png| png.len()))
    }
    /// 调用方已证明像素和 PNG 的同源关系；这里补几何、格式及编码大小准入，不重复解码。
    pub(crate) fn from_png(image: Image, png: Pixels) -> Result<Self> {
        if crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
            || !png.starts_with(b"\x89PNG\r\n\x1a\n")
            || png.len() > 64 * 1024 * 1024
        {
            return Err(Error::new(E_INVALIDARG, "Invalid formal PNG image"));
        }
        Ok(Self {
            image,
            png: Some(png),
        })
    }
    /// 仅在后台使用。保留原 PNG，不能经 PBGRA 再反预乘改变 straight RGB。
    pub(crate) fn png(&self, cancel: &AtomicBool) -> Result<Pixels> {
        check(cancel)?;
        let png = match &self.png {
            Some(png) => png.clone(),
            None => export::png_raw(&self.image, cancel)?,
        };
        check(cancel)?;
        Ok(png)
    }
    /// 只物化内联数据；PNG 字节在共享预算中保留，不访问路径、网络或用户 profile。
    pub(crate) fn inline(reference: &str, cancel: &AtomicBool) -> Result<Self> {
        let Some(encoded) = reference.strip_prefix("data:image/png;base64,") else {
            return super::session_image::inline(reference, cancel).map(Self::from);
        };
        Self::png_base64(encoded, cancel)
    }
    /// worker 复用同份已验证 PNG；不构造临时 data URL，也不经 PBGRA 反推原始颜色。
    pub(crate) fn png_base64(encoded: &str, cancel: &AtomicBool) -> Result<Self> {
        let png = super::loom_output_inline::decode_bytes("image/png", encoded, cancel)?;
        let _apartment = super::Apartment::new()?;
        let image = Image::from_encoded(&png)?;
        check(cancel)?;
        Self::from_png(image, png)
    }
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Formal PNG image cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "loom_image_value_tests.rs"]
mod tests;
