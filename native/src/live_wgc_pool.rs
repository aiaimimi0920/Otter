//! 尺寸变化同时换代两帧池和捕获会话，使静止源也交付完整的新尺寸首帧。
use super::{lifetime, resize, signal, Error, Session, SizeInt32};
use std::sync::Arc;
use windows::{
    core::IInspectable,
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::*,
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
};

pub(super) fn start(
    resources: &mut lifetime::Resources,
    frame_token: &mut Option<i64>,
    device: &IDirect3DDevice,
    item: &GraphicsCaptureItem,
    signal: &Arc<signal::Signal>,
    size: SizeInt32,
) -> Result<(), Error> {
    // 先交给原 owner；任何 API 失败仍由 Session::close 关闭部分建成的资源。
    resources.pool = Some(Direct3D11CaptureFramePool::CreateFreeThreaded(
        device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        size,
    )?);
    let pool = resources.pool.as_ref().unwrap();
    resources.session = Some(pool.CreateCaptureSession(item)?);
    let wake = signal.clone();
    *frame_token = Some(pool.FrameArrived(&TypedEventHandler::<
        Direct3D11CaptureFramePool,
        IInspectable,
    >::new(move |_, _| {
        wake.frame();
        Ok(())
    }))?);
    let session = resources.session.as_ref().unwrap();
    session.SetIsCursorCaptureEnabled(false)?;
    signal.check()?;
    session.StartCapture()?;
    Ok(())
}

fn stop(resources: &mut lifetime::Resources, frame_token: &mut Option<i64>) -> Result<(), Error> {
    if let (Some(pool), Some(token)) = (&resources.pool, *frame_token) {
        pool.RemoveFrameArrived(token)?;
        *frame_token = None;
    }
    if let Some(session) = &resources.session {
        session.Close()?;
        resources.session.take();
    }
    if let Some(pool) = &resources.pool {
        pool.Close()?;
        resources.pool.take();
    }
    Ok(())
}

impl Session {
    pub(super) fn recreate(&mut self, next: SizeInt32) -> Result<(), Error> {
        self.signal.check()?;
        // 仅 Recreate 原池会等待源下一次重绘；恢复时的唯一帧已被丢弃，静止源可永久停住。
        // 旧帧已在调用方 Close；仍用同 item/device/Signal/交付源，不重开业务会话或序号。
        resize::recreate(
            &mut self.size,
            &mut self._budget,
            next,
            crate::pixels::reserve,
            || {
                stop(&mut self.resources, &mut self.frame_token)?;
                start(
                    &mut self.resources,
                    &mut self.frame_token,
                    &self.winrt,
                    &self.item,
                    &self.signal,
                    next,
                )
            },
        )
    }
}

#[cfg(test)]
#[path = "live_wgc_pool_runtime_tests.rs"]
mod runtime_tests;
