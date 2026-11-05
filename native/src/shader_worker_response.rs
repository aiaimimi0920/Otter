//! raw RGBA 成功帧与有界编译诊断分离；未知状态、伪造尺寸和不完整像素不能发布。
use super::*;
use crate::shader_angle::{Error, Frame};
const MAX_DIAGNOSTIC: usize = 4096;
fn failure(error: &Error) -> (u32, u32, &str) {
    match error {
        Error::Compile(message) => (1, 0, message),
        Error::Link(message) => (2, 0, message),
        Error::Layout => (3, 0, ""),
        Error::Budget => (4, 0, ""),
        Error::Library => (5, 0, ""),
        Error::Context => (6, 0, ""),
        Error::Cancelled => (7, 0, ""),
        Error::Api(code) => (8, *code, ""),
        Error::Poisoned => (9, 0, ""),
        Error::ContextBusy => (10, 0, ""),
    }
}
pub(crate) fn write_response(
    writer: &mut impl Write,
    result: &Result<Frame, Error>,
) -> io::Result<()> {
    let (kind, code, width, height, bytes) = match result {
        Ok(frame) => {
            if pixel_count(frame.width, frame.height)? != frame.rgba.len() {
                return Err(invalid());
            }
            (0, 0, frame.width, frame.height, &*frame.rgba)
        }
        Err(error) => {
            let (kind, code, message) = failure(error);
            let mut end = message.len().min(MAX_DIAGNOSTIC);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            (kind, code, 0, 0, &message.as_bytes()[..end])
        }
    };
    writer.write_all(b"OSR1")?;
    for value in [kind, code, width, height, bytes.len() as u32] {
        write_u32(writer, value)?;
    }
    writer.write_all(bytes)
}
pub(crate) fn read_response(
    reader: &mut impl Read,
    expected: [u32; 2],
) -> io::Result<Result<Frame, Error>> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"OSR1" {
        return Err(invalid());
    }
    let kind = read_u32(reader)?;
    let code = read_u32(reader)?;
    let width = read_u32(reader)?;
    let height = read_u32(reader)?;
    let count = read_u32(reader)? as usize;
    if kind == 0 {
        if code != 0 || [width, height] != expected || pixel_count(width, height)? != count {
            return Err(invalid());
        }
        let mut lease = crate::pixels::reserve(count).ok_or_else(invalid)?;
        let mut pixels = Vec::new();
        pixels.try_reserve_exact(count).map_err(io::Error::other)?;
        if !lease.grow_to(pixels.capacity()) {
            return Err(invalid());
        }
        pixels.resize(count, 0);
        reader.read_exact(&mut pixels)?;
        return Ok(Ok(Frame {
            width,
            height,
            rgba: crate::pixels::Pixels::from_reserved(pixels, lease),
        }));
    }
    if kind > 10
        || width != 0
        || height != 0
        || count > MAX_DIAGNOSTIC
        || (kind != 8 && code != 0)
        || (kind == 8 && code == 0)
        || (kind > 2 && count != 0)
    {
        return Err(invalid());
    }
    let mut bytes = vec![0; count];
    reader.read_exact(&mut bytes)?;
    let message = String::from_utf8(bytes).map_err(|_| invalid())?;
    // 与后端同样不允许诊断注入终端控制字符；不回显完整 shader 或图片。
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid());
    }
    Ok(Err(match kind {
        1 => Error::Compile(message),
        2 => Error::Link(message),
        3 => Error::Layout,
        4 => Error::Budget,
        5 => Error::Library,
        6 => Error::Context,
        7 => Error::Cancelled,
        8 => Error::Api(code),
        9 => Error::Poisoned,
        10 => Error::ContextBusy,
        _ => return Err(invalid()),
    }))
}
