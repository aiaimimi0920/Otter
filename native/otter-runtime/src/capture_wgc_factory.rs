//! Borrow WGC factories only within current apartments; do not reuse static pointers surviving the final COM owner's exit.
use windows::{
    core::{factory, Interface, Type},
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
};

pub(super) fn supported() -> windows::core::Result<bool> {
    let factory = factory::<GraphicsCaptureSession, IGraphicsCaptureSessionStatics>()?;
    let mut value = false;
    // windows-rs static-method caches outlive apartment exit; release local references before this round's RoUninitialize.
    unsafe { (factory.vtable().IsSupported)(factory.as_raw(), &mut value).ok()? };
    Ok(value)
}

pub(super) fn pool(
    device: &IDirect3DDevice,
    format: DirectXPixelFormat,
    buffers: i32,
    size: SizeInt32,
) -> windows::core::Result<Direct3D11CaptureFramePool> {
    let factory = factory::<Direct3D11CaptureFramePool, IDirect3D11CaptureFramePoolStatics2>()?;
    let mut result = std::ptr::null_mut();
    // Same ABI/ownership as generated bindings: FramePool takes the sole returned reference on success, and failures propagate unchanged.
    unsafe {
        (factory.vtable().CreateFreeThreaded)(
            factory.as_raw(),
            device.as_raw(),
            format,
            buffers,
            size,
            &mut result,
        )
        .and_then(|| Direct3D11CaptureFramePool::from_abi(result))
    }
}

#[cfg(test)]
#[path = "capture_wgc_factory_tests.rs"]
mod tests;
