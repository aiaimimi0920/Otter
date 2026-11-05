//! 显式请求的一次性显示器 FP16 捕获；独立设备、两帧池，完成后关闭而非常驻。
use crate::{
    capture_color::{self, ScRgb},
    capture_readback,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use windows::{
    core::Interface,
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::{HMODULE, S_OK},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE, Direct3D11::*, Dxgi::IDXGIDevice, Gdi::HMONITOR,
        },
        System::WinRT::{Direct3D11::*, Graphics::Capture::IGraphicsCaptureItemInterop, *},
    },
};
#[path = "capture_wgc_lifetime.rs"]
mod lifetime;
#[path = "live_wgc.rs"]
pub mod live;
#[derive(Debug)]
pub enum Error {
    InvalidMonitor,
    Unsupported,
    Dimensions,
    Resized,
    Budget,
    Cancelled,
    Timeout,
    Api(windows::core::Error),
    Readback(capture_readback::Error),
}
/// 必须由显式获授权的截图请求调用，并在后台 MTA 线程执行；不弹 picker、不修改显示设置。
/// 保留系统捕获边框，不请求关闭边框权限；回调不得执行 UI 等待或启动另一捕获。
pub fn monitor<T>(
    monitor: HMONITOR,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    capture(monitor, None, cancel, consume)
}
/// region 使用目标显示器内部的物理像素；跨显示器选区须由上层单独处理，不隐式截断。
pub fn monitor_region<T>(
    monitor: HMONITOR,
    region: capture_readback::Region,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    capture(monitor, Some(region), cancel, consume)
}
fn capture<T>(
    monitor: HMONITOR,
    region: Option<capture_readback::Region>,
    cancel: &AtomicBool,
    consume: impl FnOnce(&ScRgb<'_>) -> Result<T, capture_color::Error>,
) -> Result<T, Error> {
    check(cancel)?;
    if monitor.0.is_null() {
        return Err(Error::InvalidMonitor);
    }
    let _apartment = lifetime::Apartment::new().map_err(Error::Api)?;
    if !GraphicsCaptureSession::IsSupported().map_err(Error::Api)? {
        return Err(Error::Unsupported);
    }
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .map_err(Error::Api)?;
    let item: GraphicsCaptureItem =
        unsafe { interop.CreateForMonitor(monitor).map_err(Error::Api)? };
    let size = item.Size().map_err(Error::Api)?;
    validate_region(size, region)?;
    let _pool_budget = crate::pixels::reserve(pool_bytes(size)?).ok_or(Error::Budget)?;
    check(cancel)?;
    let (device, context, winrt) = device().map_err(Error::Api)?;
    let mut resources = lifetime::Resources::default();
    resources.pool = Some(
        Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt,
            DirectXPixelFormat::R16G16B16A16Float,
            2,
            size,
        )
        .map_err(Error::Api)?,
    );
    let pool = resources.pool.as_ref().unwrap();
    resources.session = Some(pool.CreateCaptureSession(&item).map_err(Error::Api)?);
    let session = resources.session.as_ref().unwrap();
    session
        .SetIsCursorCaptureEnabled(false)
        .map_err(Error::Api)?;
    check(cancel)?;
    session.StartCapture().map_err(Error::Api)?;
    let frame = lifetime::Frame(wait(cancel, Duration::from_secs(1), || {
        pending(pool.TryGetNextFrame())
    })?);
    check(cancel)?;
    if !same_size(size, frame.0.ContentSize().map_err(Error::Api)?) {
        return Err(Error::Resized);
    }
    let surface = frame.0.Surface().map_err(Error::Api)?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(Error::Api)?;
    let texture: ID3D11Texture2D = unsafe { access.GetInterface().map_err(Error::Api)? };
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe {
        texture.GetDesc(&mut desc);
    }
    if desc.Width != size.Width as u32 || desc.Height != size.Height as u32 {
        return Err(Error::Resized);
    }
    // 每次调用独立创建 context，不发布给其它线程；帧 guard 在整个复制/映射期间持有池帧。
    let output = unsafe {
        match region {
            Some(region) => {
                capture_readback::with_region(&context, &texture, region, cancel, consume)
            }
            None => capture_readback::with_frame(&context, &texture, cancel, consume),
        }
    }
    .map_err(Error::Readback)?;
    check(cancel)?;
    drop(texture);
    drop(access);
    drop(surface);
    drop(frame);
    drop(resources);
    drop(winrt);
    drop(context);
    drop(device);
    Ok(output)
}
fn device() -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext, IDirect3DDevice)> {
    unsafe {
        let (mut device, mut context) = (None, None);
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let device = device.ok_or_else(windows::core::Error::empty)?;
        let context = context.ok_or_else(windows::core::Error::empty)?;
        let multithread: ID3D11Multithread = context.cast()?;
        // 返回的是此前状态，不是成功标志；本次独立设备始终启用保护。
        let _ = multithread.SetMultithreadProtected(true);
        let dxgi: IDXGIDevice = device.cast()?;
        let winrt = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;
        Ok((device, context, winrt))
    }
}
fn pool_bytes(size: SizeInt32) -> Result<usize, Error> {
    if size.Width <= 0 || size.Height <= 0 {
        return Err(Error::Dimensions);
    }
    crate::viewport::checked_image_bytes(size.Width as u32, size.Height as u32)
        .and_then(|n| n.checked_mul(4))
        .ok_or(Error::Dimensions)
}
fn validate_region(size: SizeInt32, region: Option<capture_readback::Region>) -> Result<(), Error> {
    pool_bytes(size)?;
    if let Some(region) = region {
        if region.width == 0
            || region.height == 0
            || region
                .left
                .checked_add(region.width)
                .is_none_or(|right| right > size.Width as u32)
            || region
                .top
                .checked_add(region.height)
                .is_none_or(|bottom| bottom > size.Height as u32)
        {
            return Err(Error::Dimensions);
        }
    }
    Ok(())
}
fn same_size(expected: SizeInt32, actual: SizeInt32) -> bool {
    expected.Width == actual.Width && expected.Height == actual.Height
}
fn pending<T>(result: windows::core::Result<T>) -> windows::core::Result<Option<T>> {
    match result {
        Ok(frame) => Ok(Some(frame)),
        // windows-core 0.60 将成功的空接口表示为 Error::empty / S_OK，不能吞掉 E_POINTER。
        Err(error) if error.code() == S_OK => Ok(None),
        Err(error) => Err(error),
    }
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
    mut next: impl FnMut() -> windows::core::Result<Option<T>>,
) -> Result<T, Error> {
    let start = Instant::now();
    loop {
        check(cancel)?;
        if let Some(frame) = next().map_err(Error::Api)? {
            return Ok(frame);
        }
        if start.elapsed() >= limit {
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[cfg(test)]
#[path = "capture_wgc_tests.rs"]
mod tests;
