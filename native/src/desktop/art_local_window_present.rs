//! 本地回复呈现事务：视口/宿主前后复核，原 FIFO ticket 按结算→焦点→默认动作完成。
use super::*;
impl Window {
    fn check_viewport(&self, expected: Option<crate::surface_viewport::Viewport>) -> Result<()> {
        if let (Some(window), Some(expected)) = (&self.window, expected) {
            if window.viewport()? != expected || window.viewport_epoch() != self.input.epoch {
                return Err(Error::new(E_FAIL, "Local Art viewport changed"));
            }
        }
        Ok(())
    }
    pub(super) fn present(
        &mut self,
        frame: crate::surface_javascript::Frame,
        host: &impl Fn() -> bool,
        visible: bool,
    ) -> Result<()> {
        if self.result.is_some() {
            return Err(Error::new(E_FAIL, "Unconsumed local Art result"));
        }
        let expected = if self.input.spatial {
            self.input.viewport
        } else {
            frame.viewport
        };
        self.check_viewport(expected)?;
        let images = frame
            .images
            .into_iter()
            .map(|(id, image)| Ok((id, crate::surface_image_paint::Bitmap::new(image)?)))
            .collect::<Result<_>>()?;
        if let Some(window) = &self.window {
            window.replace_javascript_frame(frame.scene, images)?;
        } else {
            let window = SurfaceWindow::with_images(frame.scene, 640, 480, images)?;
            window.enable_javascript_keyboard();
            window.bind_edits(&self.input.view)?;
            self.window = Some(window);
        }
        self.check_viewport(expected)?;
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art host changed"));
        }
        let window = self.window.as_ref().unwrap();
        if !window.is_alive() {
            return Err(Error::new(E_FAIL, "Local Art window closed"));
        }
        let ticket = self.input.ticket.take();
        if let Some(id) = &ticket {
            window.settle_javascript_ticket(id)?;
        }
        if visible {
            window.apply_javascript_focus(&frame.focus, self.input.focus)?;
        }
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art focus host changed"));
        }
        if let Some(id) = ticket {
            window.complete_javascript_edit(
                &id,
                frame.default_allowed.map(|allowed| allowed && visible),
            )?;
        } else if frame.default_allowed.is_some() {
            return Err(Error::new(E_FAIL, "Unsolicited local Art keyboard result"));
        }
        self.input.spatial = false;
        self.has_frame = true;
        self.schedule(
            frame.animation_pending,
            frame.timer_delay_ms,
            frame.resize_observed,
        );
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art result host changed"));
        }
        self.result = frame.result;
        Ok(())
    }
}
