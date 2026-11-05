//! 编译/分配前的输入准入及固定 ABI 布局；名字只在校验后生成 HLSL。
use super::*;
pub(super) fn image_bytes(width: u32, height: u32) -> Result<usize> {
    if width > 8192 || height > 8192 {
        return Err(Error::Layout);
    }
    crate::viewport::checked_image_bytes(width, height)
        .filter(|bytes| *bytes <= 128 * 1024 * 1024)
        .ok_or(Error::Layout)
}
pub(super) fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with("otter_")
        || !value
            .bytes()
            .enumerate()
            .all(|(i, c)| c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit()))
    {
        return Err(Error::Layout);
    }
    Ok(())
}
pub(super) fn validate(request: &Request<'_>) -> Result<()> {
    image_bytes(request.width, request.height)?;
    if request.abi != ABI
        || request.source.is_empty()
        || request.source.len() > MAX_SOURCE
        || request.source.contains('\0')
        || request.textures.is_empty()
        || request.textures.len() > 32
        || request.uniforms.len() > 256
        || !request
            .textures
            .iter()
            .any(|t| t.name == "input" && t.width == request.width && t.height == request.height)
    {
        return Err(Error::Layout);
    }
    let mut total = 0usize;
    for (i, texture) in request.textures.iter().enumerate() {
        name(texture.name)?;
        if image_bytes(texture.width, texture.height)? != texture.rgba.len()
            || request.textures[..i].iter().any(|t| t.name == texture.name)
        {
            return Err(Error::Layout);
        }
        total = total
            .checked_add(texture.rgba.len())
            .filter(|n| *n <= MAX_TEXTURE_BYTES)
            .ok_or(Error::Budget)?;
    }
    for (i, uniform) in request.uniforms.iter().enumerate() {
        name(uniform.name)?;
        if !uniform.value.is_finite()
            || request.uniforms[..i].iter().any(|u| u.name == uniform.name)
            || request.textures.iter().any(|t| t.name == uniform.name)
        {
            return Err(Error::Layout);
        }
    }
    Ok(())
}
pub(super) fn allocate(count: usize) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    allocate_with(count, crate::pixels::reserve)
}
pub(super) fn allocate_with(
    count: usize,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<(Vec<u8>, crate::byte_budget::Lease)> {
    let mut lease = reserve(count).ok_or(Error::Budget)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(count).map_err(|_| Error::Budget)?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::Budget);
    }
    bytes.resize(count, 0);
    Ok((bytes, lease))
}
/// b0/c0 为尺寸；每个标量单独占一个 float4，避免平台/编译器紧凑排列歧义。
pub(super) fn constants(request: &Request<'_>) -> Vec<u8> {
    let mut bytes = vec![0; (request.uniforms.len() + 1) * 16];
    bytes[0..4].copy_from_slice(&(request.width as f32).to_le_bytes());
    bytes[4..8].copy_from_slice(&(request.height as f32).to_le_bytes());
    for (i, value) in request.uniforms.iter().enumerate() {
        bytes[(i + 1) * 16..(i + 1) * 16 + 4].copy_from_slice(&value.value.to_le_bytes());
    }
    bytes
}
