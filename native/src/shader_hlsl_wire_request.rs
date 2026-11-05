//! 请求头先限长，纹理只在剩余 body/预算足够时分配；私有输入析构清零。
use super::*;
use zeroize::Zeroizing;
struct Loaded {
    name: String,
    width: u32,
    height: u32,
    rgba: Zeroizing<Vec<u8>>,
    _lease: crate::byte_budget::Lease,
}
pub(in crate::shader_hlsl) struct Owned {
    pub backend: Backend,
    width: u32,
    height: u32,
    source: String,
    textures: Vec<Loaded>,
    uniforms: Vec<(String, f32)>,
}
impl Owned {
    pub fn with_request<T>(&self, run: impl FnOnce(&Request<'_>) -> T) -> T {
        let textures = self
            .textures
            .iter()
            .map(|t| Texture {
                name: &t.name,
                width: t.width,
                height: t.height,
                rgba: &t.rgba,
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
        run(&Request {
            abi: ABI,
            source: &self.source,
            width: self.width,
            height: self.height,
            textures: &textures,
            uniforms: &uniforms,
        })
    }
}
pub(in crate::shader_hlsl) fn write_request(
    writer: &mut impl Write,
    value: &Request<'_>,
    selected: Backend,
) -> io::Result<()> {
    validate(value).map_err(|_| invalid())?;
    let mut body = value.source.len();
    for t in value.textures {
        body = body
            .checked_add(16 + t.name.len() + t.rgba.len())
            .ok_or_else(invalid)?;
    }
    for u in value.uniforms {
        body = body.checked_add(8 + u.name.len()).ok_or_else(invalid)?;
    }
    writer.write_all(b"OHQ1")?;
    for n in [
        u32::try_from(body).map_err(|_| invalid())?,
        backend(selected),
        value.width,
        value.height,
        value.source.len() as u32,
        value.textures.len() as u32,
        value.uniforms.len() as u32,
    ] {
        number(writer, n)?;
    }
    writer.write_all(value.source.as_bytes())?;
    for t in value.textures {
        for n in [t.name.len() as u32, t.width, t.height, t.rgba.len() as u32] {
            number(writer, n)?;
        }
        writer.write_all(t.name.as_bytes())?;
        writer.write_all(t.rgba)?;
    }
    for u in value.uniforms {
        number(writer, u.name.len() as u32)?;
        writer.write_all(&u.value.to_le_bytes())?;
        writer.write_all(u.name.as_bytes())?;
    }
    Ok(())
}
pub(in crate::shader_hlsl) fn read_request(reader: &mut impl Read) -> io::Result<Owned> {
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != b"OHQ1" {
        return Err(invalid());
    }
    let body_bytes = read_number(reader)? as usize;
    let backend = read_backend(read_number(reader)?)?;
    let width = read_number(reader)?;
    let height = read_number(reader)?;
    contract::image_bytes(width, height).map_err(|_| invalid())?;
    let source_bytes = read_number(reader)? as usize;
    let textures_count = read_number(reader)? as usize;
    let uniforms_count = read_number(reader)? as usize;
    if source_bytes == 0
        || source_bytes > MAX_SOURCE
        || textures_count == 0
        || textures_count > 32
        || uniforms_count > 256
        || body_bytes > MAX_TEXTURE_BYTES + MAX_SOURCE + 65536
        || body_bytes < source_bytes + textures_count * 17 + uniforms_count * 9
    {
        return Err(invalid());
    }
    let mut body = reader.take(body_bytes as u64);
    let source = text(&mut body, source_bytes, MAX_SOURCE)?;
    if source.contains('\0') {
        return Err(invalid());
    }
    let mut textures: Vec<Loaded> = Vec::with_capacity(textures_count);
    let mut total = 0usize;
    for i in 0..textures_count {
        let len = read_number(&mut body)? as usize;
        let width = read_number(&mut body)?;
        let height = read_number(&mut body)?;
        let bytes = read_number(&mut body)? as usize;
        if bytes != contract::image_bytes(width, height).map_err(|_| invalid())? {
            return Err(invalid());
        }
        total = total
            .checked_add(bytes)
            .filter(|n| *n <= MAX_TEXTURE_BYTES)
            .ok_or_else(invalid)?;
        let name = text(&mut body, len, 128)?;
        contract::name(&name).map_err(|_| invalid())?;
        if textures.iter().any(|t| t.name == name)
            || bytes as u64 + ((textures_count - i - 1) * 21 + uniforms_count * 9) as u64
                > body.limit()
        {
            return Err(invalid());
        }
        let (pixels, lease) = contract::allocate(bytes).map_err(|_| invalid())?;
        let mut rgba = Zeroizing::new(pixels);
        body.read_exact(&mut rgba)?;
        textures.push(Loaded {
            name,
            width,
            height,
            rgba,
            _lease: lease,
        });
    }
    let mut uniforms: Vec<(String, f32)> = Vec::with_capacity(uniforms_count);
    for _ in 0..uniforms_count {
        let len = read_number(&mut body)? as usize;
        let value = f32::from_bits(read_number(&mut body)?);
        let name = text(&mut body, len, 128)?;
        contract::name(&name).map_err(|_| invalid())?;
        if !value.is_finite()
            || textures.iter().any(|t| t.name == name)
            || uniforms.iter().any(|u| u.0 == name)
        {
            return Err(invalid());
        }
        uniforms.push((name, value));
    }
    if body.limit() != 0 {
        return Err(invalid());
    }
    let value = Owned {
        backend,
        width,
        height,
        source,
        textures,
        uniforms,
    };
    value.with_request(validate).map_err(|_| invalid())?;
    Ok(value)
}
