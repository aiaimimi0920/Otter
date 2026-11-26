//! Single FP16/BGRA8 texture readback; context-exclusive callers own source frames, without device caching/capture startup here.
use crate::capture_color::{self, ScRgb};
#[path = "capture_readback_sdr.rs"]
pub mod sdr;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, DXGI_ERROR_WAS_STILL_DRAWING},
    },
};
#[derive(Debug)]
pub enum Error {
    Layout,
    Device,
    Budget,
    Cancelled,
    Timeout,
    Api(windows::core::Error),
    Conversion(capture_color::Error),
}
/// Source-texture-local physical pixels with exclusive right/bottom boundaries; no monitor-global coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Region {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}
struct Mapping<'a> {
    context: &'a ID3D11DeviceContext,
    texture: &'a ID3D11Texture2D,
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        unsafe {
            self.context.Unmap(self.texture, 0);
        }
    }
}
/// Callbacks borrow only this mapping and may not retain raw pointers; return values cannot borrow mapped data.
///
/// # Safety
/// Exclusively own immediate context throughout the call/callback; source textures must not be mapped/concurrently modified.
/// Source textures/devices must come from trusted D3D11 APIs; callers own source-texture budgets/capture-session lifecycles.
pub unsafe fn with_frame<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    read(context, source, cancel, consume, crate::pixels::reserve)
}
/// Copy only selection into staging, never read back whole source textures first.
///
/// # Safety
/// As with with_frame: exclusively own immediate context during calls, without mapped/concurrently modified source textures.
pub unsafe fn with_region<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: Region,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    read_region(
        context,
        source,
        Some(region),
        cancel,
        consume,
        crate::pixels::reserve,
    )
}
unsafe fn read<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<T, Error> {
    read_region(context, source, None, cancel, consume, reserve)
}
unsafe fn read_region<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: Option<Region>,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<T, Error> {
    read_mapped(
        context,
        source,
        region,
        Format::Fp16,
        cancel,
        reserve,
        |bytes, width, height, stride| {
            let frame = ScRgb::new(bytes, width, height, stride).map_err(Error::Conversion)?;
            consume(&frame).map_err(Error::Conversion)
        },
    )
}

#[derive(Clone, Copy)]
pub enum Format {
    Fp16,
    Bgra8,
}
impl Format {
    pub fn pixel_bytes(self) -> usize {
        match self {
            Self::Fp16 => 8,
            Self::Bgra8 => 4,
        }
    }
    fn dxgi(self) -> DXGI_FORMAT {
        match self {
            Self::Fp16 => DXGI_FORMAT_R16G16B16A16_FLOAT,
            Self::Bgra8 => DXGI_FORMAT_B8G8R8A8_UNORM,
        }
    }
}

// The consumer must finish while the scoped mapping and staging lease are still alive.
unsafe fn read_mapped<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    region: Option<Region>,
    format: Format,
    cancel: &AtomicBool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
    consume: impl FnOnce(&[u8], u32, u32, usize) -> Result<T, Error>,
) -> Result<T, Error> {
    check(cancel)?;
    if context.GetType() != D3D11_DEVICE_CONTEXT_IMMEDIATE {
        return Err(Error::Device);
    }
    let device = source.GetDevice().map_err(Error::Api)?;
    let owner = context.GetDevice().map_err(Error::Api)?;
    if device.cast::<IUnknown>().map_err(Error::Api)?
        != owner.cast::<IUnknown>().map_err(Error::Api)?
    {
        return Err(Error::Device);
    }
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    source.GetDesc(&mut desc);
    validate(&desc, format)?;
    let area = crop(&desc, region)?;
    desc.Width = area.right - area.left;
    desc.Height = area.bottom - area.top;
    let bytes = validate(&desc, format)?;
    let mut lease = reserve(bytes).ok_or(Error::Budget)?;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .map_err(Error::Api)?;
    let staging = staging.ok_or(Error::Device)?;
    check(cancel)?;
    context.CopySubresourceRegion(&staging, 0, 0, 0, 0, source, 0, Some(&area));
    context.Flush();
    let mapped = wait(cancel, Duration::from_millis(250), || {
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(
            &staging,
            0,
            D3D11_MAP_READ,
            D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
            Some(&mut mapped),
        )?;
        Ok(mapped)
    })?;
    // Establish guards before checking returned layouts/fallible conversions; panic also automatically Unmaps.
    let _mapping = Mapping {
        context,
        texture: &staging,
    };
    check(cancel)?;
    let length = (mapped.RowPitch as usize)
        .checked_mul(desc.Height as usize)
        .ok_or(Error::Layout)?;
    if mapped.pData.is_null()
        || (mapped.RowPitch as usize) < desc.Width as usize * format.pixel_bytes()
        || length > isize::MAX as usize
    {
        return Err(Error::Layout);
    }
    // D3D11 exposes no physical VRAM allocation size; admit logical pixels first, then supplement readback leases using known mapped spans.
    if !lease.grow_to(length) {
        return Err(Error::Budget);
    }
    let bytes = std::slice::from_raw_parts(mapped.pData.cast::<u8>(), length);
    let result = consume(bytes, desc.Width, desc.Height, mapped.RowPitch as usize)?;
    check(cancel)?;
    Ok(result)
}
fn validate(desc: &D3D11_TEXTURE2D_DESC, format: Format) -> Result<usize, Error> {
    if desc.Format != format.dxgi()
        || desc.MipLevels != 1
        || desc.ArraySize != 1
        || desc.SampleDesc.Count != 1
        || desc.SampleDesc.Quality != 0
    {
        return Err(Error::Layout);
    }
    crate::viewport::checked_image_bytes(desc.Width, desc.Height)
        .and_then(|bgra| (bgra / 4).checked_mul(format.pixel_bytes()))
        .ok_or(Error::Layout)
}
fn crop(desc: &D3D11_TEXTURE2D_DESC, region: Option<Region>) -> Result<D3D11_BOX, Error> {
    let region = region.unwrap_or(Region {
        left: 0,
        top: 0,
        width: desc.Width,
        height: desc.Height,
    });
    let right = region.left.checked_add(region.width).ok_or(Error::Layout)?;
    let bottom = region.top.checked_add(region.height).ok_or(Error::Layout)?;
    if region.width == 0 || region.height == 0 || right > desc.Width || bottom > desc.Height {
        return Err(Error::Layout);
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
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn wait<T>(
    cancel: &AtomicBool,
    limit: Duration,
    mut map: impl FnMut() -> windows::core::Result<T>,
) -> Result<T, Error> {
    let start = Instant::now();
    // Spin/yield briefly before sleeping (matching `live_texture_readback`): GPU DMA copies typically finish
    // within ~50-200 us, whereas an immediate `sleep(1ms)` stalls for an entire Windows timer tick (~1-15.6 ms).
    let mut attempts = 0u32;
    loop {
        check(cancel)?;
        match map() {
            Ok(value) => return Ok(value), // Once mapped, callers install guards before checking cancellation.
            Err(error) if error.code() == DXGI_ERROR_WAS_STILL_DRAWING => (),
            Err(error) => return Err(Error::Api(error)),
        }
        if start.elapsed() >= limit {
            return Err(Error::Timeout);
        }
        if attempts < 8 {
            std::hint::spin_loop();
        } else if attempts < 16 {
            std::thread::yield_now();
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
        attempts = attempts.saturating_add(1);
    }
}
#[cfg(test)]
#[path = "capture_readback_tests.rs"]
mod tests;
