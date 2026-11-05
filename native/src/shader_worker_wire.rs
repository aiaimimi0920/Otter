//! Shader 专属有界帧；只通过受信父进程继承的 duplex pipe，不复用图片 PBGRA 协议。
use crate::{
    shader_angle::{Backend, Request},
    shader_process::Invocation,
};
#[cfg(test)]
use std::io::{Read, Write};
use std::{io, os::windows::ffi::OsStrExt};
#[cfg(test)]
#[path = "shader_worker_request.rs"]
mod request;
#[cfg(test)]
#[path = "shader_worker_response.rs"]
mod response;
#[cfg(test)]
pub(crate) use request::{read_request, write_request};
#[cfg(test)]
pub(crate) use response::{read_response, write_response};
pub(crate) const MAX_SOURCE: usize = 1024 * 1024;
pub(crate) const MAX_TEXTURE_BYTES: usize = 256 * 1024 * 1024;
const MAX_BODY: usize = MAX_TEXTURE_BYTES + 4 * 1024 * 1024;
pub(crate) fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid shader worker frame")
}
pub(crate) fn pixel_count(width: u32, height: u32) -> io::Result<usize> {
    // 保留旧 ShaderRenderer 的 8192 / 32 MP（含 4K）准入，而非图片通用上限。
    if width > 8192 || height > 8192 {
        return Err(invalid());
    }
    crate::viewport::checked_image_bytes(width, height)
        .filter(|count| *count <= 128 * 1024 * 1024)
        .ok_or_else(invalid)
}
fn identifier(name: &str) -> io::Result<()> {
    crate::shader_angle::name(name)
        .map(|_| ())
        .map_err(|_| invalid())
}
fn source(text: &str) -> io::Result<()> {
    if text.is_empty() || text.len() > MAX_SOURCE || text.contains('\0') {
        return Err(invalid());
    }
    Ok(())
}
pub(crate) fn validate(width: u32, height: u32, request: &Request<'_>) -> io::Result<usize> {
    pixel_count(width, height)?;
    source(request.vertex)?;
    source(request.fragment)?;
    if request.textures.is_empty()
        || request.textures.len() > 32
        || request.uniforms.len() > 256
        || !request
            .textures
            .iter()
            .any(|item| item.name == "input" && item.width == width && item.height == height)
    {
        return Err(invalid());
    }
    let mut names = Vec::new();
    let mut total = 0usize;
    let mut encoded = request.vertex.len() + request.fragment.len();
    for texture in request.textures {
        identifier(texture.name)?;
        if names.contains(&texture.name)
            || pixel_count(texture.width, texture.height)? != texture.rgba.len()
        {
            return Err(invalid());
        }
        names.push(texture.name);
        total = total
            .checked_add(texture.rgba.len())
            .filter(|sum| *sum <= MAX_TEXTURE_BYTES)
            .ok_or_else(invalid)?;
        encoded += 16 + texture.name.len() + texture.rgba.len();
    }
    for uniform in request.uniforms {
        identifier(uniform.name)?;
        if !uniform.value.is_finite() || names.contains(&uniform.name) {
            return Err(invalid());
        }
        names.push(uniform.name);
        encoded += 8 + uniform.name.len();
    }
    Ok(encoded)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub body: u32,
    pub backend: u32,
    pub width: u32,
    pub height: u32,
    pub path_units: u32,
    pub vertex: u32,
    pub fragment: u32,
    pub textures: u32,
    pub uniforms: u32,
}
impl Header {
    pub(crate) fn for_invocation(value: &Invocation<'_>) -> io::Result<Self> {
        let encoded = validate(value.width, value.height, &value.request)?;
        let units = value
            .bundle
            .directory
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>();
        if units.is_empty() || units.len() > 32767 || units.contains(&0) {
            return Err(invalid());
        }
        let header = Self {
            body: (80 + units.len() * 2 + encoded) as u32,
            backend: match value.backend {
                Backend::D3d11 => 0,
                Backend::Warp => 1,
            },
            width: value.width,
            height: value.height,
            path_units: units.len() as u32,
            vertex: value.request.vertex.len() as u32,
            fragment: value.request.fragment.len() as u32,
            textures: value.request.textures.len() as u32,
            uniforms: value.request.uniforms.len() as u32,
        };
        header.validate()?;
        Ok(header)
    }
    fn validate(&self) -> io::Result<()> {
        pixel_count(self.width, self.height)?;
        let minimum = 80u64
            + u64::from(self.path_units) * 2
            + u64::from(self.vertex)
            + u64::from(self.fragment)
            + u64::from(self.textures) * 17
            + u64::from(self.uniforms) * 9;
        if self.body as usize > MAX_BODY
            || u64::from(self.body) < minimum
            || self.backend > 1
            || self.path_units == 0
            || self.path_units > 32767
            || self.vertex == 0
            || self.vertex as usize > MAX_SOURCE
            || self.fragment == 0
            || self.fragment as usize > MAX_SOURCE
            || self.textures == 0
            || self.textures > 32
            || self.uniforms > 256
        {
            return Err(invalid());
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.validate()?;
        writer.write_all(b"OSQ1")?;
        for value in [
            self.body,
            self.backend,
            self.width,
            self.height,
            self.path_units,
            self.vertex,
            self.fragment,
            self.textures,
            self.uniforms,
        ] {
            write_u32(writer, value)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"OSQ1" {
            return Err(invalid());
        }
        let header = Self {
            body: read_u32(reader)?,
            backend: read_u32(reader)?,
            width: read_u32(reader)?,
            height: read_u32(reader)?,
            path_units: read_u32(reader)?,
            vertex: read_u32(reader)?,
            fragment: read_u32(reader)?,
            textures: read_u32(reader)?,
            uniforms: read_u32(reader)?,
        };
        header.validate()?;
        Ok(header)
    }
}
#[cfg(test)]
fn write_u32(writer: &mut impl Write, value: u32) -> io::Result<()> {
    writer.write_all(&value.to_le_bytes())
}
#[cfg(test)]
fn read_u32(reader: &mut impl Read) -> io::Result<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}
#[cfg(test)]
#[path = "shader_worker_wire_tests.rs"]
mod tests;
