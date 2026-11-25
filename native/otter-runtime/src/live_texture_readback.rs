//! 显式 Live 快照的 BGRA 读回；不捕获桌面，不在连续呈现路径复制 CPU 像素。
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
    core::{IUnknown, Interface},
    Win32::Graphics::{
        Direct3D11::*,
        Dxgi::{Common::*, DXGI_ERROR_WAS_STILL_DRAWING},
    },
};

struct CachedStaging {
    device: usize,
    width: u32,
    height: u32,
    texture: ID3D11Texture2D,
    // staging 跨调用保留时预算同样保留；字段顺序保证 COM 先于租约释放。
    lease: crate::byte_budget::Lease,
}

/// 单流独占的复用的 CPU staging 纹理缓存；断开或尺寸变化时立即替换，不跨流共享。
#[derive(Default)]
pub struct Scratch(Option<CachedStaging>);
impl Scratch {
    pub fn clear(&mut self) {
        self.0 = None;
    }
    pub fn read(
        &mut self,
        frame: Arc<Frame<Texture>>,
        cancel: &AtomicBool,
    ) -> Result<SdrFrame, Error> {
        unsafe {
            read_inner(
                &frame.resource,
                cancel,
                crate::pixels::reserve,
                Some(&mut self.0),
            )
        }
    }
    #[cfg(test)]
    fn raw_staging(&self) -> Option<*mut core::ffi::c_void> {
        self.0.as_ref().map(|entry| entry.texture.as_raw())
    }
}

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
    unsafe { read_inner(&frame.resource, cancel, crate::pixels::reserve, None) }
}
unsafe fn read_inner(
    frame: &Texture,
    cancel: &AtomicBool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    scratch: Option<&mut Option<CachedStaging>>,
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
    let device_id = device.cast::<IUnknown>().map_err(Error::Api)?.as_raw() as usize;
    let mut transient = None;
    let slot = scratch.unwrap_or(&mut transient);
    if slot.as_ref().is_none_or(|entry| {
        entry.device != device_id || entry.width != desc.Width || entry.height != desc.Height
    }) {
        // 替换前先释放旧规格，避免新旧 staging 同时占预算。
        *slot = None;
        let lease = reserve(size).ok_or(Error::Budget)?;
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.MiscFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        let mut created = None;
        device
            .CreateTexture2D(&desc, None, Some(&mut created))
            .map_err(Error::Api)?;
        *slot = Some(CachedStaging {
            device: device_id,
            width: desc.Width,
            height: desc.Height,
            texture: created.ok_or(Error::Layout)?,
            lease,
        });
    }
    let staging = slot.as_mut().unwrap();
    let result = (|| {
        let mut output_lease = reserve(size).ok_or(Error::Budget)?;
        check(cancel)?;
        context.CopyResource(&staging.texture, texture);
        context.Flush();
        let mapped = wait(cancel, Duration::from_millis(250), || {
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            context.Map(
                &staging.texture,
                0,
                D3D11_MAP_READ,
                D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32,
                Some(&mut mapped),
            )?;
            Ok(mapped)
        })?;
        // 映射成功立即安装 guard；取消、预算不足、布局异常和 panic 都必须 Unmap。
        let _mapping = Mapping(&context, &staging.texture);
        check(cancel)?;
        let row = desc.Width as usize * 4;
        let span = (mapped.RowPitch as usize)
            .checked_mul(desc.Height as usize)
            .ok_or(Error::Layout)?;
        if mapped.pData.is_null() || (mapped.RowPitch as usize) < row || span > isize::MAX as usize
        {
            return Err(Error::Layout);
        }
        if !staging.lease.grow_to(span) {
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
    })();
    if result.is_err() {
        // Map 已解除后再淘汰失败资源，尤其不能保留 padding 扩容失败的欠额租约。
        *slot = None;
    }
    result
}
fn wait<T>(
    cancel: &AtomicBool,
    limit: Duration,
    mut map: impl FnMut() -> windows::core::Result<T>,
) -> Result<T, Error> {
    let start = Instant::now();
    let mut attempts = 0u32;
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
        // 先短自旋/让出以吸收微秒级 GPU 复制完成，避免立即落入 ~15.6ms 的默认睡眠粒度。
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
#[path = "live_texture_readback_tests.rs"]
mod tests;
