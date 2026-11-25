//! Single-window remote pixel consumer; fixed permits cannot be replaced by readmission, and failures close the display owner.
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
            sample(
                &surface.composition,
                size.width as usize / 2,
                size.height as usize / 2,
            )
        };
        Ok(Some((surface.metadata, pixel)))
    }
    /// Destroy before the owning HWND; a media Session may bind only one consumer window.
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
    /// Callers own BeginPaint/EndPaint; without new frames, redraw existing bitmaps only, without reupload.
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
            // Release CPU frames/budgets immediately after upload; retain GPU bitmaps until resize or closure.
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
    /// Release only this window's display resources; upper session owners still stop/reclaim media threads and control leases.
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

// Shared by application fixtures even when the Live-only suites are disabled.
#[cfg(test)]
pub(super) unsafe fn sample(composition: &Composition, x: usize, y: usize) -> [u8; 4] {
    let device: ID3D11Device = composition.state.chain.GetDevice().unwrap();
    let context = device.GetImmediateContext().unwrap();
    let source: ID3D11Texture2D = composition.state.chain.GetBuffer(0).unwrap();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    source.GetDesc(&mut desc);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    device
        .CreateTexture2D(&desc, None, Some(&mut staging))
        .unwrap();
    let staging = staging.unwrap();
    context.CopyResource(&staging, &source);
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    context
        .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        .unwrap();
    let mut result = [0; 4];
    std::ptr::copy_nonoverlapping(
        (mapped.pData as *const u8).add(y * mapped.RowPitch as usize + x * 4),
        result.as_mut_ptr(),
        4,
    );
    context.Unmap(&staging, 0);
    result
}
