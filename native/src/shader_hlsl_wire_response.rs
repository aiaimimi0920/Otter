//! 成功像素/实际后端与失败分离；有界 UTF-8 诊断、严格状态和尺寸。
use super::*;
fn stage(value: Stage) -> u32 {
    match value {
        Stage::Device => 1,
        Stage::Resource => 2,
        Stage::Pipeline => 3,
        Stage::Submit => 4,
        Stage::Readback => 5,
    }
}
fn read_stage(value: u32) -> io::Result<Stage> {
    match value {
        1 => Ok(Stage::Device),
        2 => Ok(Stage::Resource),
        3 => Ok(Stage::Pipeline),
        4 => Ok(Stage::Submit),
        5 => Ok(Stage::Readback),
        _ => Err(invalid()),
    }
}
fn selected(value: SelectedBackend) -> u32 {
    match value {
        SelectedBackend::D3d12 => 0,
        SelectedBackend::D3d11 => 1,
        SelectedBackend::Warp => 2,
    }
}
pub(in crate::shader_hlsl) fn write_response(
    writer: &mut impl Write,
    result: &Result<Frame>,
) -> io::Result<()> {
    let (kind, backend, stage, code, width, height, bytes) = match result {
        Ok(frame) => {
            if contract::image_bytes(frame.width, frame.height).map_err(|_| invalid())?
                != frame.rgba.len()
            {
                return Err(invalid());
            }
            (
                0,
                selected(frame.backend),
                0,
                0,
                frame.width,
                frame.height,
                &*frame.rgba,
            )
        }
        Err(error) => {
            let (kind, stage, code, message) = match error {
                Error::Layout => (1, 0, 0, ""),
                Error::Budget => (2, 0, 0, ""),
                Error::Unsupported => (3, 0, 0, ""),
                Error::Isolation => (4, 0, 0, ""),
                Error::Cancelled => (5, 0, 0, ""),
                Error::TimedOut => (6, 0, 0, ""),
                Error::Compile(message) => (7, 0, 0, message.as_str()),
                Error::Api { stage: s, code } => (8, stage(*s), *code as u32, ""),
            };
            let mut end = message.len().min(4096);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            (kind, 0, stage, code, 0, 0, &message.as_bytes()[..end])
        }
    };
    writer.write_all(b"OHR1")?;
    for n in [
        kind,
        backend,
        stage,
        code,
        width,
        height,
        bytes.len() as u32,
    ] {
        number(writer, n)?;
    }
    writer.write_all(bytes)
}
pub(in crate::shader_hlsl) fn read_response(
    reader: &mut impl Read,
    expected: [u32; 2],
) -> io::Result<Result<Frame>> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"OHR1" {
        return Err(invalid());
    }
    let kind = read_number(reader)?;
    let backend = read_number(reader)?;
    let stage = read_number(reader)?;
    let code = read_number(reader)?;
    let width = read_number(reader)?;
    let height = read_number(reader)?;
    let bytes = read_number(reader)? as usize;
    if kind == 0 {
        if stage != 0
            || code != 0
            || [width, height] != expected
            || contract::image_bytes(width, height).map_err(|_| invalid())? != bytes
        {
            return Err(invalid());
        }
        let backend = match backend {
            0 => SelectedBackend::D3d12,
            1 => SelectedBackend::D3d11,
            2 => SelectedBackend::Warp,
            _ => return Err(invalid()),
        };
        let (mut pixels, lease) = contract::allocate(bytes).map_err(|_| invalid())?;
        reader.read_exact(&mut pixels)?;
        return Ok(Ok(Frame {
            width,
            height,
            rgba: Pixels::from_reserved(pixels, lease),
            backend,
        }));
    }
    if kind > 8
        || backend != 0
        || width != 0
        || height != 0
        || bytes > 4096
        || (kind != 7 && bytes != 0)
        || (kind != 8 && (stage != 0 || code != 0))
        || (kind == 8 && (code as i32) >= 0)
    {
        return Err(invalid());
    }
    let message = if bytes == 0 {
        String::new()
    } else {
        text(reader, bytes, 4096)?
    };
    if message
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid());
    }
    Ok(Err(match kind {
        1 => Error::Layout,
        2 => Error::Budget,
        3 => Error::Unsupported,
        4 => Error::Isolation,
        5 => Error::Cancelled,
        6 => Error::TimedOut,
        7 => Error::Compile(message),
        8 => Error::Api {
            stage: read_stage(stage)?,
            code: code as i32,
        },
        _ => return Err(invalid()),
    }))
}
