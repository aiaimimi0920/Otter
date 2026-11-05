//! 父侧共用接收门：先校验 child 尺寸再领取 Pixels，完整退出/EOF/撤销后才发布。
use super::*;
#[cfg(test)]
#[path = "surface_svg_wire_tests.rs"]
mod tests;
use crate::{
    live_recovery_pipe::Pipe, surface_image_worker_process as process,
    surface_image_worker_wire as wire,
};
use std::{
    io::{self, Read, Write},
    time::{Duration, Instant},
};
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid image worker response")
}
pub(super) fn run(
    bytes: &[u8],
    codec: wire::Codec,
    expected: Option<(u32, u32)>,
    stopped: &impl Fn() -> bool,
    spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<process::Child>,
) -> io::Result<Image> {
    run_sized(bytes, codec, expected, stopped, spawn).map(|(image, _)| image)
}
pub(super) fn run_sized(
    bytes: &[u8],
    codec: wire::Codec,
    expected: Option<(u32, u32)>,
    stopped: &impl Fn() -> bool,
    spawn: impl FnOnce(std::os::windows::io::OwnedHandle, Instant) -> io::Result<process::Child>,
) -> io::Result<(Image, [f32; 2])> {
    if bytes.is_empty() || bytes.len() > wire::MAX_INPUT || stopped() {
        return Err(invalid());
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    let (mut pipe, endpoint) = Pipe::pair()?;
    let child = spawn(endpoint.into_handle(), deadline)?;
    let (width, height, logical_size, pixels, lease) = {
        let mut channel = pipe.cancellable_io(deadline, child.handle(), stopped);
        channel.write_all(codec.request())?;
        channel.write_all(&(bytes.len() as u32).to_le_bytes())?;
        channel.write_all(bytes)?;
        let mut header = [0; 16];
        channel.read_exact(&mut header)?;
        let (width, height, count) = wire::response_for(codec, &header)?;
        if expected.is_some_and(|size| size != (width, height)) {
            return Err(invalid());
        }
        let logical_size = if codec == wire::Codec::Svg {
            let mut metadata = [0; 8];
            channel.read_exact(&mut metadata)?;
            wire::logical_size(
                [
                    f32::from_le_bytes(metadata[..4].try_into().unwrap()),
                    f32::from_le_bytes(metadata[4..].try_into().unwrap()),
                ],
                [width, height],
            )?
        } else {
            [width as f32, height as f32]
        };
        let (mut pixels, lease) = pixel_buffer(count).map_err(io::Error::other)?;
        channel.read_exact(&mut pixels)?;
        (width, height, logical_size, pixels, lease)
    };
    child.finish(deadline, stopped)?;
    let mut extra = [0];
    match pipe.io(Some(deadline), None).read(&mut extra) {
        Ok(0) => {}
        Err(error) if closed(&error) => {}
        _ => return Err(invalid()),
    }
    if stopped() {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "Image cancelled",
        ));
    }
    Ok((
        Image {
            width,
            height,
            pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
        },
        logical_size,
    ))
}
fn closed(error: &io::Error) -> bool {
    use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED};
    error.kind() == io::ErrorKind::BrokenPipe
        || error
            .get_ref()
            .and_then(|e| e.downcast_ref::<windows::core::Error>())
            .is_some_and(|e| {
                [ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED]
                    .iter()
                    .any(|code| e.code() == windows::core::HRESULT::from_win32(code.0))
            })
}
