//! Live 帧池纹理的独立快照：仅 GPU 复制，预算随共享帧存活，不发布可被池复用的纹理。
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{Direct3D11::*, Dxgi::Common::*},
};
#[path = "live_texture_readback.rs"]
pub mod readback;
pub struct Texture {
    texture: ID3D11Texture2D,
    _lease: crate::byte_budget::Lease,
    pub width: u32,
    pub height: u32,
    pub format: DXGI_FORMAT,
}
impl Texture {
    /// 仅借用。呈现者必须持有整个共享帧，不得只克隆 COM 指针而提前释放预算 owner。
    pub fn texture(&self) -> &ID3D11Texture2D {
        &self.texture
    }
}
#[derive(Debug)]
pub enum Error {
    Dimensions,
    Device,
    Budget,
    Cancelled,
    Api(windows::core::Error),
    Delivery(crate::live_frames::Error),
}
fn bytes(desc: &D3D11_TEXTURE2D_DESC) -> Result<usize, Error> {
    if desc.MipLevels != 1
        || desc.ArraySize != 1
        || desc.SampleDesc.Count != 1
        || desc.SampleDesc.Quality != 0
    {
        return Err(Error::Dimensions);
    }
    let base =
        crate::viewport::checked_image_bytes(desc.Width, desc.Height).ok_or(Error::Dimensions)?;
    match desc.Format {
        DXGI_FORMAT_B8G8R8A8_UNORM => Ok(base),
        DXGI_FORMAT_R16G16B16A16_FLOAT => base.checked_mul(2).ok_or(Error::Dimensions),
        _ => Err(Error::Dimensions),
    }
}
/// # Safety
/// 调用者串行拥有 immediate context，或已启用并遵守 D3D 多线程保护；源不能正在映射。
/// GPU 命令按同设备 context 顺序消费；此函数不代表跨设备共享/fence 同步已经完成。
pub unsafe fn snapshot(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
) -> Result<Texture, Error> {
    copy(context, source, None, cancel)
}
/// # Safety
/// 与 snapshot 相同；region 为源纹理内物理像素，必须完整包含，不允许隐式裁剪。
pub unsafe fn snapshot_region(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: crate::capture_readback::Region,
    cancel: &AtomicBool,
) -> Result<Texture, Error> {
    copy(context, source, Some(region), cancel)
}
unsafe fn copy(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: Option<crate::capture_readback::Region>,
    cancel: &AtomicBool,
) -> Result<Texture, Error> {
    check(cancel)?;
    if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
        return Err(Error::Device);
    }
    let device = context.GetDevice().map_err(Error::Api)?;
    let source_device = source.GetDevice().map_err(Error::Api)?;
    if device.cast::<IUnknown>().map_err(Error::Api)?
        != source_device.cast::<IUnknown>().map_err(Error::Api)?
    {
        return Err(Error::Device);
    }
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    source.GetDesc(&mut desc);
    bytes(&desc)?;
    let full = region.is_none();
    let region = region.unwrap_or(crate::capture_readback::Region {
        left: 0,
        top: 0,
        width: desc.Width,
        height: desc.Height,
    });
    let area = region_box(region, desc.Width, desc.Height)?;
    desc.Width = region.width;
    desc.Height = region.height;
    let lease = crate::pixels::reserve(bytes(&desc)?).ok_or(Error::Budget)?;
    let destination = D3D11_TEXTURE2D_DESC {
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
        ..desc
    };
    let mut texture = None;
    device
        .CreateTexture2D(&destination, None, Some(&mut texture))
        .map_err(Error::Api)?;
    let texture = texture.ok_or(Error::Device)?;
    check(cancel)?;
    if full {
        context.CopyResource(&texture, source);
    } else {
        context.CopySubresourceRegion(&texture, 0, 0, 0, 0, source, 0, Some(&area));
    }
    device.GetDeviceRemovedReason().map_err(Error::Api)?;
    check(cancel)?;
    Ok(Texture {
        texture,
        _lease: lease,
        width: desc.Width,
        height: desc.Height,
        format: desc.Format,
    })
}
pub(crate) fn region_box(
    region: crate::capture_readback::Region,
    width: u32,
    height: u32,
) -> Result<D3D11_BOX, Error> {
    let right = region
        .left
        .checked_add(region.width)
        .ok_or(Error::Dimensions)?;
    let bottom = region
        .top
        .checked_add(region.height)
        .ok_or(Error::Dimensions)?;
    if region.width == 0 || region.height == 0 || right > width || bottom > height {
        return Err(Error::Dimensions);
    }
    Ok(D3D11_BOX {
        left: region.left,
        top: region.top,
        front: 0,
        right,
        bottom,
        back: 1,
    })
}
/// # Safety
/// 与 snapshot 相同。无消费者时不分配、不复制；准备期间最后一个消费者离开则丢弃快照。
pub unsafe fn publish(
    destination: &crate::live_frames::Source<Texture>,
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
) -> Result<u64, Error> {
    check(cancel)?;
    if !destination.has_consumers() {
        return Err(Error::Delivery(crate::live_frames::Error::NoConsumers));
    }
    destination
        .publish(snapshot(context, source, cancel)?)
        .map_err(Error::Delivery)
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "live_texture_tests.rs"]
mod tests;
