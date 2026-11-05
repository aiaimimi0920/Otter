//! 解码前 RIFF/静态图/内外尺寸准入；不代替 child 的 Job 工作内存限制。
pub(crate) use crate::surface_image_worker_wire::MAX_INPUT;
use std::io;
pub(crate) fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid or unsupported static WebP",
    )
}
fn le24(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
}
fn size(width: u32, height: u32) -> io::Result<(u32, u32)> {
    crate::surface_image_worker_wire::pixel_count(width, height).map_err(|_| invalid())?;
    Ok((width, height))
}
pub(crate) fn inspect(bytes: &[u8]) -> io::Result<(u32, u32)> {
    if bytes.len() < 20
        || bytes.len() > MAX_INPUT
        || &bytes[..4] != b"RIFF"
        || &bytes[8..12] != b"WEBP"
    {
        return Err(invalid());
    }
    let end = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if end.checked_add(8) != Some(bytes.len()) {
        return Err(invalid());
    }
    let (mut at, mut count, mut canvas, mut image) = (12usize, 0, None, None);
    while at < bytes.len() {
        count += 1;
        let head = bytes.get(at..at + 8).ok_or_else(invalid)?;
        let len = u32::from_le_bytes(head[4..8].try_into().unwrap()) as usize;
        let start = at + 8;
        let end = start.checked_add(len).ok_or_else(invalid)?;
        let body = bytes.get(start..end).ok_or_else(invalid)?;
        at = end
            .checked_add(len % 2)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(invalid)?;
        if count > 256 {
            return Err(invalid());
        }
        let dimensions = match &head[..4] {
            b"ANIM" | b"ANMF" => return Err(invalid()),
            b"VP8X" => {
                if count != 1 || body.len() != 10 || body[0] & 2 != 0 || canvas.is_some() {
                    return Err(invalid());
                }
                canvas = Some(size(le24(&body[4..7]) + 1, le24(&body[7..10]) + 1)?);
                None
            }
            b"VP8 " => {
                if body.len() < 10 || body[0] & 1 != 0 || &body[3..6] != b"\x9d\x01\x2a" {
                    return Err(invalid());
                }
                Some(size(
                    u32::from(u16::from_le_bytes([body[6], body[7]]) & 0x3fff),
                    u32::from(u16::from_le_bytes([body[8], body[9]]) & 0x3fff),
                )?)
            }
            b"VP8L" => {
                if body.len() < 5 || body[0] != 0x2f {
                    return Err(invalid());
                }
                let bits = u32::from_le_bytes(body[1..5].try_into().unwrap());
                if bits >> 29 != 0 {
                    return Err(invalid());
                }
                Some(size((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)?)
            }
            _ => None,
        };
        if let Some(dimensions) = dimensions {
            if image.replace(dimensions).is_some() {
                return Err(invalid());
            }
        }
    }
    let image = image.ok_or_else(invalid)?;
    if canvas.is_some_and(|v| v != image) {
        return Err(invalid());
    }
    Ok(image)
}
