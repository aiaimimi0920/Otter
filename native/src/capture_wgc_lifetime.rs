//! WinRT 必须显式 Close；局部声明顺序保证帧先归还、session/pool 先关闭、Apartment 最后退出。
use super::*;
pub(super) struct Apartment;
impl Apartment {
    pub(super) fn new() -> windows::core::Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
#[derive(Default)]
pub(super) struct Resources {
    pub(super) pool: Option<Direct3D11CaptureFramePool>,
    pub(super) session: Option<GraphicsCaptureSession>,
}
impl Drop for Resources {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = session.Close();
        }
        if let Some(pool) = self.pool.take() {
            let _ = pool.Close();
        }
    }
}
pub(super) struct Frame(pub(super) Direct3D11CaptureFrame);
impl Drop for Frame {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
