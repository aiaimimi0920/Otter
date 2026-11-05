//! 单请求流式编码/解码；纹理逐项先计数、领取预算再读，私有图像缓冲析构时清零。
use super::*;
use crate::{
    shader_angle::{Fingerprint, Identity, Texture, Uniform},
    shader_process::TrustedBundle,
};
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use zeroize::Zeroizing;
struct LoadedTexture {
    name: String,
    width: u32,
    height: u32,
    rgba: Zeroizing<Vec<u8>>,
    _lease: crate::byte_budget::Lease,
}
pub(crate) struct OwnedRequest {
    bundle: TrustedBundle,
    header: Header,
    vertex: String,
    fragment: String,
    textures: Vec<LoadedTexture>,
    uniforms: Vec<(String, f32)>,
}
impl OwnedRequest {
    pub(crate) fn with_invocation<T>(&self, use_request: impl FnOnce(Invocation<'_>) -> T) -> T {
        let textures = self
            .textures
            .iter()
            .map(|item| Texture {
                name: &item.name,
                width: item.width,
                height: item.height,
                rgba: &item.rgba,
            })
            .collect::<Vec<_>>();
        let uniforms = self
            .uniforms
            .iter()
            .map(|(name, value)| Uniform {
                name,
                value: *value,
            })
            .collect::<Vec<_>>();
        use_request(Invocation {
            bundle: &self.bundle,
            backend: if self.header.backend == 0 {
                Backend::D3d11
            } else {
                Backend::Warp
            },
            width: self.header.width,
            height: self.header.height,
            request: Request {
                vertex: &self.vertex,
                fragment: &self.fragment,
                textures: &textures,
                uniforms: &uniforms,
            },
        })
    }
}
pub(crate) fn write_request(
    writer: &mut impl Write,
    value: &Invocation<'_>,
    header: &Header,
) -> io::Result<()> {
    // 调用方已准入，发送时再复核固定头；不组装第二份全图编码缓冲。
    let expected = Header::for_invocation(value)?;
    if expected != *header {
        return Err(invalid());
    }
    header.write(writer)?;
    for identity in [value.bundle.identity.egl, value.bundle.identity.gles] {
        writer.write_all(&identity.bytes.to_le_bytes())?;
        writer.write_all(&identity.sha256)?;
    }
    for unit in value.bundle.directory.as_os_str().encode_wide() {
        writer.write_all(&unit.to_le_bytes())?;
    }
    writer.write_all(value.request.vertex.as_bytes())?;
    writer.write_all(value.request.fragment.as_bytes())?;
    for texture in value.request.textures {
        for field in [
            texture.name.len() as u32,
            texture.width,
            texture.height,
            texture.rgba.len() as u32,
        ] {
            write_u32(writer, field)?;
        }
        writer.write_all(texture.name.as_bytes())?;
        writer.write_all(texture.rgba)?;
    }
    for uniform in value.request.uniforms {
        write_u32(writer, uniform.name.len() as u32)?;
        writer.write_all(&uniform.value.to_le_bytes())?;
        writer.write_all(uniform.name.as_bytes())?;
    }
    Ok(())
}
fn text(reader: &mut impl Read, length: usize, maximum: usize) -> io::Result<String> {
    if length == 0 || length > maximum {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(io::Error::other)?;
    bytes.resize(length, 0);
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| invalid())
}
fn fingerprint(reader: &mut impl Read) -> io::Result<Fingerprint> {
    let mut length = [0; 8];
    let mut sha256 = [0; 32];
    reader.read_exact(&mut length)?;
    reader.read_exact(&mut sha256)?;
    let bytes = u64::from_le_bytes(length);
    if bytes == 0 || bytes > 32 * 1024 * 1024 {
        return Err(invalid());
    }
    Ok(Fingerprint { bytes, sha256 })
}
pub(crate) fn read_request(reader: &mut impl Read) -> io::Result<OwnedRequest> {
    let header = Header::read(reader)?;
    let mut body = reader.take(u64::from(header.body));
    let identity = Identity {
        egl: fingerprint(&mut body)?,
        gles: fingerprint(&mut body)?,
    };
    let mut units = Vec::with_capacity(header.path_units as usize);
    for _ in 0..header.path_units {
        let mut bytes = [0; 2];
        body.read_exact(&mut bytes)?;
        units.push(u16::from_le_bytes(bytes));
    }
    if units.contains(&0) {
        return Err(invalid());
    }
    let directory = PathBuf::from(OsString::from_wide(&units));
    // 只允许可信父侧的绝对本地路径；不把 pipe 内容解析成相对 cwd 或网络 DLL 路径。
    if !directory.is_absolute() {
        return Err(invalid());
    }
    let bundle = crate::shader_candidate::from_worker(&directory, identity)?;
    let vertex = text(&mut body, header.vertex as usize, MAX_SOURCE)?;
    let fragment = text(&mut body, header.fragment as usize, MAX_SOURCE)?;
    source(&vertex)?;
    source(&fragment)?;
    let mut textures = Vec::with_capacity(header.textures as usize);
    let mut total = 0usize;
    for index in 0..header.textures {
        let length = read_u32(&mut body)? as usize;
        let width = read_u32(&mut body)?;
        let height = read_u32(&mut body)?;
        let count = read_u32(&mut body)? as usize;
        if count != pixel_count(width, height)? {
            return Err(invalid());
        }
        total = total
            .checked_add(count)
            .filter(|sum| *sum <= MAX_TEXTURE_BYTES)
            .ok_or_else(invalid)?;
        let name = text(&mut body, length, 128)?;
        identifier(&name)?;
        // 短帧不能先触发大纹理分配；为剩余描述符和 uniform 保留最低必需尾部。
        let tail = u64::from(header.textures - index - 1) * 21 + u64::from(header.uniforms) * 9;
        if count as u64 + tail > body.limit() {
            return Err(invalid());
        }
        if textures
            .iter()
            .any(|item: &LoadedTexture| item.name == name)
        {
            return Err(invalid());
        }
        let mut lease = crate::pixels::reserve(count).ok_or_else(invalid)?;
        let mut rgba = Zeroizing::new(Vec::new());
        rgba.try_reserve_exact(count).map_err(io::Error::other)?;
        if !lease.grow_to(rgba.capacity()) {
            return Err(invalid());
        }
        rgba.resize(count, 0);
        body.read_exact(&mut rgba)?;
        textures.push(LoadedTexture {
            name,
            width,
            height,
            rgba,
            _lease: lease,
        });
    }
    let mut uniforms = Vec::with_capacity(header.uniforms as usize);
    for _ in 0..header.uniforms {
        let length = read_u32(&mut body)? as usize;
        let mut bytes = [0; 4];
        body.read_exact(&mut bytes)?;
        let value = f32::from_le_bytes(bytes);
        let name = text(&mut body, length, 128)?;
        identifier(&name)?;
        if !value.is_finite()
            || textures.iter().any(|item| item.name == name)
            || uniforms
                .iter()
                .any(|(existing, _): &(String, f32)| existing == &name)
        {
            return Err(invalid());
        }
        uniforms.push((name, value));
    }
    if body.limit() != 0 {
        return Err(invalid());
    }
    let request = OwnedRequest {
        bundle,
        header,
        vertex,
        fragment,
        textures,
        uniforms,
    };
    request.with_invocation(|invocation| {
        if Header::for_invocation(&invocation)?.body != request.header.body {
            return Err(invalid());
        }
        Ok(())
    })?;
    Ok(request)
}
