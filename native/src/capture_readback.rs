//! 单次 FP16 纹理读回：独占 context 的调用方拥有源帧；本层不缓存设备或启动捕获。
use crate::capture_color::{self, ScRgb};
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
/// 源纹理局部物理像素，右/下边界不包含在选区内；不接受显示器全局坐标。
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
/// 回调只能借用本次映射，不得保存裸指针；返回值不能借用映射数据。
///
/// # Safety
/// 调用期间必须独占 immediate context（包含回调），源纹理不得被映射或并发修改。
/// 源纹理与设备必须来自可信 D3D11 API；调用方负责源纹理预算和捕获会话生命周期。
pub unsafe fn with_frame<T>(
    context: &ID3D11DeviceContext,
    source: &ID3D11Texture2D,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    read(context, source, cancel, consume, crate::pixels::reserve)
}
/// 只复制选区到 staging；不会先读回整张源纹理。
///
/// # Safety
/// 与 with_frame 相同：调用期间独占 immediate context，源纹理不得被映射或并发修改。
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
    validate(&desc)?;
    let area = crop(&desc, region)?;
    desc.Width = area.right - area.left;
    desc.Height = area.bottom - area.top;
    let bytes = validate(&desc)?;
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
    // 必须先建立 guard，再检查返回布局或执行可失败的转换；panic 同样自动 Unmap。
    let _mapping = Mapping {
        context,
        texture: &staging,
    };
    check(cancel)?;
    let length = (mapped.RowPitch as usize)
        .checked_mul(desc.Height as usize)
        .ok_or(Error::Layout)?;
    if mapped.pData.is_null() || mapped.RowPitch < desc.Width * 8 || length > isize::MAX as usize {
        return Err(Error::Layout);
    }
    // D3D11 不公开物理显存分配尺寸；先准入逻辑像素，再按已知映射跨度补足读回租约。
    if !lease.grow_to(length) {
        return Err(Error::Budget);
    }
    let bytes = std::slice::from_raw_parts(mapped.pData.cast::<u8>(), length);
    let frame = ScRgb::new(bytes, desc.Width, desc.Height, mapped.RowPitch as usize)
        .map_err(Error::Conversion)?;
    let result = consume(&frame).map_err(Error::Conversion)?;
    check(cancel)?;
    Ok(result)
}
fn validate(desc: &D3D11_TEXTURE2D_DESC) -> Result<usize, Error> {
    if desc.Format != DXGI_FORMAT_R16G16B16A16_FLOAT
        || desc.MipLevels != 1
        || desc.ArraySize != 1
        || desc.SampleDesc.Count != 1
        || desc.SampleDesc.Quality != 0
    {
        return Err(Error::Layout);
    }
    crate::viewport::checked_image_bytes(desc.Width, desc.Height)
        .and_then(|n| n.checked_mul(2))
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
    loop {
        check(cancel)?;
        match map() {
            Ok(value) => return Ok(value), // 已映射时由调用方先安装 guard，再检查取消。
            Err(error) if error.code() == DXGI_ERROR_WAS_STILL_DRAWING => (),
            Err(error) => return Err(Error::Api(error)),
        }
        if start.elapsed() >= limit {
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[cfg(test)]
#[path = "capture_readback_tests.rs"]
mod tests;
