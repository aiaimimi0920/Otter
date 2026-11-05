//! 新 HLSL 私有 v1 帧，不复用含 ANGLE bundle/GLSL 的 OSQ1；逐项计费和流式传输。
use super::*;
use std::io::{self, Read, Write};
#[path = "shader_hlsl_wire_request.rs"]
mod request;
#[path = "shader_hlsl_wire_response.rs"]
mod response;
pub(super) use request::{read_request, write_request};
pub(super) use response::{read_response, write_response};
pub(super) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid HLSL worker frame")
}
fn number(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
fn read_number(reader: &mut impl Read) -> io::Result<u32> {
    let mut b = [0; 4];
    reader.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}
fn backend(value: Backend) -> u32 {
    match value {
        Backend::Auto => 0,
        Backend::D3d12 => 1,
        Backend::D3d11 => 2,
        Backend::Warp => 3,
        Backend::Software => 4,
    }
}
fn read_backend(value: u32) -> io::Result<Backend> {
    match value {
        0 => Ok(Backend::Auto),
        1 => Ok(Backend::D3d12),
        2 => Ok(Backend::D3d11),
        3 => Ok(Backend::Warp),
        4 => Ok(Backend::Software),
        _ => Err(invalid()),
    }
}
fn text(reader: &mut impl Read, bytes: usize, maximum: usize) -> io::Result<String> {
    if bytes == 0 || bytes > maximum {
        return Err(invalid());
    }
    let mut data = vec![0; bytes];
    reader.read_exact(&mut data)?;
    String::from_utf8(data).map_err(|_| invalid())
}
#[cfg(test)]
#[path = "shader_hlsl_wire_tests.rs"]
mod tests;
