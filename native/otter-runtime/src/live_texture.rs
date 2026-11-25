//! Independent snapshots of Live frame-pool textures: GPU copies only, with budget tied to texture lifetime; return to the bounded session pool only after the last consumer releases.
use std::{
    mem::ManuallyDrop,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{Direct3D11::*, Dxgi::Common::*},
};
#[path = "live_texture_pool.rs"]
pub mod pool;
#[path = "live_texture_readback.rs"]
pub mod readback;
pub struct Texture {
    allocation: ManuallyDrop<pool::Allocation>,
    returner: Option<pool::Returner>,
    pub width: u32,
    pub height: u32,
    pub format: DXGI_FORMAT,
}
impl Texture {
    /// Borrow only. Presenters must retain the entire shared frame, not just clone COM pointers and prematurely release the budget owner.
    pub fn texture(&self) -> &ID3D11Texture2D {
        &self.allocation.texture
    }
}
impl Drop for Texture {
    fn drop(&mut self) {
        let allocation = unsafe { ManuallyDrop::take(&mut self.allocation) };
        if let Some(returner) = self.returner.take() {
            pool::recycle(returner, allocation);
        }
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
/// The caller serially owns the immediate context, or enables and obeys D3D multithread protection; the source must not be mapped.
/// GPU commands are consumed in same-device context order; this function does not establish completed cross-device sharing/fence synchronization.
pub unsafe fn snapshot(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
) -> Result<Texture, Error> {
    copy(context, source, None, cancel)
}
/// # Safety
/// Same as snapshot; region uses physical pixels within the source texture and must be fully contained, with no implicit clipping.
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
    copy_pooled(
        context,
        source,
        region,
        cancel,
        None,
        crate::pixels::reserve,
    )
}
pub(super) unsafe fn copy_pooled(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: Option<crate::capture_readback::Region>,
    cancel: &AtomicBool,
    pool: Option<&Arc<Mutex<pool::State>>>,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
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
    let destination = D3D11_TEXTURE2D_DESC {
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
        ..desc
    };
    let (allocation, returner) = pool::acquire(pool, &device, &destination, reserve)?;
    check(cancel)?;
    if full {
        context.CopyResource(&allocation.texture, source);
    } else {
        context.CopySubresourceRegion(&allocation.texture, 0, 0, 0, 0, source, 0, Some(&area));
    }
    device.GetDeviceRemovedReason().map_err(Error::Api)?;
    check(cancel)?;
    Ok(Texture {
        allocation: ManuallyDrop::new(allocation),
        returner,
        width: desc.Width,
        height: desc.Height,
        format: desc.Format,
    })
}
pub fn region_box(
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
/// Same as snapshot. No allocation/copy without consumers; discard the snapshot if the last consumer leaves during preparation.
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
