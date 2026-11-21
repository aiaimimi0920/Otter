//! WGC 工厂只在当前 apartment 内借用，不能复用最后一个 COM owner 退出前的静态指针。
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
    // windows-rs 的静态方法缓存跨越 apartment 退出；局部引用在本轮 RoUninitialize 前释放。
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
    // 与生成绑定相同的 ABI/所有权：成功后由 FramePool 接管唯一返回引用，失败原样传播。
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
