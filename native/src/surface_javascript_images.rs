//! 私有 mount 前图片安装：JSON只传描述，PNG沿同一受控pipe逐项传输；不开放路径/URL。
//! 不重置Session期限；输入、解码像素和target位图另在既有Canvas总预算中计费。
use super::invalid;
use crate::pixels::Pixels;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{self, Read, Write};
pub(crate) const MAX_IMAGES: usize = 8;
pub(crate) const MAX_PNG: usize = 4 * 1024 * 1024;
const TOTAL_PNG: usize = 8 * 1024 * 1024;
const TOTAL_PIXELS: usize = 16 * 1024 * 1024;

/// 只能由宿主从已核验的图输入准备结果构造；克隆PNG共享原像素租约。
#[derive(Clone)]
pub struct ImageResource {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub png: Pixels,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Header {
    pub id: String,
    pub size: [u32; 2],
    pub bytes: usize,
    pub digest: String,
}
pub(crate) fn validate(headers: &[Header]) -> io::Result<()> {
    if headers.is_empty() || headers.len() > MAX_IMAGES {
        return Err(invalid());
    }
    let (mut png, mut pixels) = (0usize, 0usize);
    for (index, header) in headers.iter().enumerate() {
        let count = super::canvas::bytes(header.size)?;
        if count == 0
            || !crate::surface_javascript_event::safe(&header.id)
            || headers[..index].iter().any(|item| item.id == header.id)
            || header.bytes == 0
            || header.bytes > MAX_PNG
            || header.digest.len() != 64
            || !header
                .digest
                .bytes()
                .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
        {
            return Err(invalid());
        }
        png = png.checked_add(header.bytes).ok_or_else(invalid)?;
        pixels = pixels.checked_add(count).ok_or_else(invalid)?;
        if png > TOTAL_PNG || pixels > TOTAL_PIXELS {
            return Err(invalid());
        }
    }
    Ok(())
}
pub(crate) fn headers(images: &[ImageResource]) -> io::Result<Vec<Header>> {
    // 先检查长度/尺寸再哈希；大的内联字符串永不进入JS heap或OJS1 JSON帧。
    let mut headers: Vec<_> = images
        .iter()
        .take(MAX_IMAGES + 1)
        .map(|image| Header {
            id: image.id.clone(),
            size: [image.width, image.height],
            bytes: image.png.len(),
            digest: "0".repeat(64),
        })
        .collect();
    validate(&headers)?;
    for (header, image) in headers.iter_mut().zip(images) {
        header.digest = format!("{:x}", Sha256::digest(&*image.png));
    }
    Ok(headers)
}
pub(crate) fn write(writer: &mut impl Write, images: &[ImageResource]) -> io::Result<()> {
    for image in images {
        writer.write_all(&image.png)?;
    }
    writer.flush()
}
pub(crate) fn read(reader: &mut impl Read, header: &Header) -> io::Result<Pixels> {
    if header.bytes == 0 || header.bytes > MAX_PNG {
        return Err(invalid());
    }
    let lease = crate::pixels::reserve(header.bytes).ok_or_else(invalid)?;
    let mut bytes = vec![0; header.bytes];
    reader.read_exact(&mut bytes)?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || format!("{:x}", Sha256::digest(&bytes)) != header.digest
    {
        return Err(invalid());
    }
    Ok(Pixels::from_reserved(bytes, lease))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Installed {
    pub sequence: u64,
}
#[cfg(test)]
#[path = "surface_javascript_images_tests.rs"]
pub(crate) mod tests;
