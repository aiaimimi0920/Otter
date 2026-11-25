//! Size changes replace both the two-frame pool and capture session so stationary sources also deliver a complete first frame at the new size.
use super::{lifetime, resize, signal, Error, Session, SizeInt32};
use std::sync::Arc;
use windows::{
    core::IInspectable,
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem},
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
    // Transfer to the original owner first; Session::close still closes partially constructed resources on any API failure.
    resources.pool = Some(super::super::factory::pool(
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
    // Match baseline Win32 capture; retain existing capture capability on older systems lacking this interface.
    if let Err(error) = session.SetIsBorderRequired(false) {
        if error.code() != windows::Win32::Foundation::E_NOINTERFACE {
            return Err(error.into());
        }
    }
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
        // Recreate alone on the original pool waits for the next source repaint; if the sole restoration frame was discarded, a stationary source may stall forever.
        // The caller already Closed the old frame; retain the same item/device/Signal/delivery source without restarting the business session or sequence.
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
