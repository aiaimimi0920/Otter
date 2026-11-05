//! 显式 Live 快照的 BGRA 读回；不捕获桌面，不缓存纹理，不在连续呈现路径复制 CPU 像素。
use super::Texture;
use crate::{capture_color::SdrFrame, live_frames::Frame};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use windows::{
    core::Interface,
    Win32::Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, DXGI_ERROR_WAS_STILL_DRAWING},
    },
};

#[derive(Debug)]
pub enum Error {
    Layout,
    Unprotected,
    Budget,
    Cancelled,
    Timeout,
    Api(windows::core::Error),
}
struct Mapping<'a>(&'a ID3D11DeviceContext, &'a ID3D11Texture2D);
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.Unmap(self.1, 0);
        }
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}

/// 后台调用；完整 frame owner 在返回前保留预算。只接受独立发布的不可变 BGRA 帧。
/// immediate context 必须启用 D3D 多线程保护，每个 API 调用由驱动串行化；
/// staging 仅本次读回使用，不把等待锁持有到 UI 呈现或采集线程上。
pub fn read(frame: Arc<Frame<Texture>>, cancel: &AtomicBool) -> Result<SdrFrame, Error> {
    unsafe { read_inner(&frame.resource, cancel, crate::pixels::reserve) }
}
unsafe fn read_inner(
    frame: &Texture,
    cancel: &AtomicBool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<SdrFrame, Error> {
    check(cancel)?;
    let texture = frame.texture();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    texture.GetDesc(&mut desc);
    if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM
        || frame.format != desc.Format
        || frame.width != desc.Width
        || frame.height != desc.Height
        || desc.MipLevels != 1
        || desc.ArraySize != 1
        || desc.SampleDesc.Count != 1
        || desc.SampleDesc.Quality != 0
    {
        return Err(Error::Layout);
    }
    let size =
        crate::viewport::checked_image_bytes(desc.Width, desc.Height).ok_or(Error::Layout)?;
    let device = texture.GetDevice().map_err(Error::Api)?;
    let context = device.GetImmediateContext().map_err(Error::Api)?;
    let protection: ID3D11Multithread = context.cast().map_err(Error::Api)?;
    if !protection.GetMultithreadProtected().as_bool() {
        return Err(Error::Unprotected);
    }
    let mut staging_lease = reserve(size).ok_or(Error::Budget)?;
    let mut output_lease = reserve(size).ok_or(Error::Budget)?;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .map_err(Error::Api)?;
    let staging = staging.ok_or(Error::Layout)?;
    check(cancel)?;
    context.CopyResource(&staging, texture);
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
    // 映射成功立即安装 guard；取消、预算不足、布局异常和 panic 都必须 Unmap。
    let _mapping = Mapping(&context, &staging);
    check(cancel)?;
    let row = desc.Width as usize * 4;
    let span = (mapped.RowPitch as usize)
        .checked_mul(desc.Height as usize)
        .ok_or(Error::Layout)?;
    if mapped.pData.is_null() || (mapped.RowPitch as usize) < row || span > isize::MAX as usize {
        return Err(Error::Layout);
    }
    if !staging_lease.grow_to(span) {
        return Err(Error::Budget);
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| Error::Budget)?;
    if !output_lease.grow_to(bytes.capacity()) {
        return Err(Error::Budget);
    }
    for y in 0..desc.Height as usize {
        check(cancel)?;
        bytes.extend_from_slice(std::slice::from_raw_parts(
            mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize),
            row,
        ));
    }
    check(cancel)?;
    Ok(SdrFrame {
        width: desc.Width,
        height: desc.Height,
        pixels: crate::pixels::Pixels::from_reserved(bytes, output_lease),
    })
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
            Ok(value) => return Ok(value),
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
#[path = "live_texture_readback_tests.rs"]
mod tests;
