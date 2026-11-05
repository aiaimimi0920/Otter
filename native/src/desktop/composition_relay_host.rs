//! 单窗口远程像素消费者；固定许可不能被重新 admit 替换，失败关闭显示 owner。
use super::*;
use crate::{
    live_relay_viewer_frames::Cursor,
    live_relay_viewer_session::{Permit, ViewerSession},
};
use std::{os::windows::io::BorrowedHandle, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    Idle,
    Frame,
    Closed,
}
pub struct Host {
    hwnd: HWND,
    media: Arc<ViewerSession>,
    permit: Permit,
    cursor: Cursor,
    pending: Option<PixelFrame>,
    business: Option<Arc<crate::live_relay_viewer_business::Business>>,
    surface: Option<Surface>,
    closed: bool,
}
impl Host {
    pub(super) fn presented_viewport(
        &self,
        size: [u32; 2],
    ) -> Option<crate::live_viewport::Viewport> {
        if self.closed || self.media.validate(&self.permit).is_err() {
            return None;
        }
        let surface = self.surface.as_ref()?;
        if surface.presented != Some(size) {
            return None;
        }
        crate::live_viewport::Viewport::fit([surface.metadata.width, surface.metadata.height], size)
    }
    #[cfg(test)]
    pub(super) fn test_frame(&mut self, size: D2D_SIZE_U) -> Result<Option<(Metadata, [u8; 4])>> {
        self.paint(size)?;
        let Some(surface) = self.surface.as_mut() else {
            return Ok(None);
        };
        surface.draw(size)?;
        let pixel = unsafe {
            super::tests::sample(
                &surface.composition,
                size.width as usize / 2,
                size.height as usize / 2,
            )
        };
        Ok(Some((surface.metadata, pixel)))
    }
    /// 必须在所属 HWND 销毁前析构；一个媒体 Session 只能绑定一个消费窗口。
    pub fn bind(hwnd: HWND, media: Arc<ViewerSession>, permit: Permit) -> Result<Self> {
        media
            .validate(&permit)
            .map_err(|_| Error::new(E_FAIL, "Remote viewer permit invalid"))?;
        Ok(Self {
            hwnd,
            media,
            cursor: Cursor {
                epoch: permit.epoch(),
                frame_id: 0,
            },
            permit,
            pending: None,
            business: None,
            surface: None,
            closed: false,
        })
    }
    pub fn change_handle(&self) -> Option<BorrowedHandle<'_>> {
        (!self.closed).then(|| self.media.change_handle())
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn business(&self) -> Option<&crate::live_relay_viewer_business::Business> {
        if self.closed || self.media.validate(&self.permit).is_err() {
            None
        } else {
            self.business.as_deref()
        }
    }
    pub(super) fn business_snapshot(
        &self,
    ) -> Option<Arc<crate::live_relay_viewer_business::Business>> {
        self.business()?;
        self.business.clone()
    }
    pub fn refresh(&mut self) -> Update {
        if self.closed {
            return Update::Closed;
        }
        if self.media.validate(&self.permit).is_err() {
            self.close();
            return Update::Closed;
        }
        match self.media.poll_update(self.cursor) {
            Ok(update) => {
                self.business = update.business;
                if let Some(frame) = update.frame {
                    self.cursor = frame.metadata().into();
                    self.pending = Some(frame);
                    Update::Frame
                } else {
                    Update::Idle
                }
            }
            Err(_) => {
                self.close();
                Update::Closed
            }
        }
    }
    /// 调用者负责 BeginPaint/EndPaint；无新帧时仅重绘既有位图，不重新上传。
    pub fn paint(&mut self, size: D2D_SIZE_U) -> Result<bool> {
        if self.refresh() == Update::Closed {
            return Ok(false);
        }
        if size.width == 0 || size.height == 0 {
            return Ok(false);
        }
        let result = self.paint_inner(size);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn paint_inner(&mut self, size: D2D_SIZE_U) -> Result<bool> {
        if let Some(frame) = self.pending.take() {
            match self.surface.as_mut() {
                Some(surface) => surface.set_frame(&frame)?,
                None => self.surface = Some(Surface::new(self.hwnd, size, &frame)?),
            }
            // 上传结束即释放 CPU 帧/预算；GPU 位图保留到更换尺寸或关闭。
        }
        if self.media.validate(&self.permit).is_err() {
            self.close();
            return Ok(false);
        }
        let Some(surface) = self.surface.as_mut() else {
            return Ok(false);
        };
        surface.draw(size)?;
        surface.composition.present()?;
        surface.presented = Some([size.width, size.height]);
        Ok(true)
    }
    /// 只释放本窗口显示资源；媒体线程与控制租约仍由上层会话 owner 停止/回收。
    pub fn close(&mut self) {
        self.closed = true;
        self.pending = None;
        self.business = None;
        self.surface = None;
    }
    #[cfg(test)]
    pub(super) fn surface_for_test(&self) -> Option<&Surface> {
        self.surface.as_ref()
    }
}
