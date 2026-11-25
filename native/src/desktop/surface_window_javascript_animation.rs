//! rAF/timers reuse application blocking-wait deadlines; contribute no timer wakes under backpressure or without an owner.
use super::*;
impl AuthorizedWindow {
    fn javascript_resize_dirty(&self) -> bool {
        self.javascript.as_ref().is_some_and(|js| {
            js.resize_observed && js.observed_viewport_epoch != self.window.viewport_epoch()
        })
    }
    pub(super) fn capture_javascript_viewport(
        &mut self,
    ) -> Result<crate::surface_viewport::Viewport> {
        // Capture message generation rather than consuming a boolean; an old reply must not clear another resize that occurred in flight.
        self.javascript.as_mut().unwrap().submitted_viewport_epoch = self.window.viewport_epoch();
        self.javascript.as_mut().unwrap().submitted_focus_epoch =
            self.window.javascript_focus_epoch();
        self.window.viewport()
    }
    fn javascript_animation_allowed(&self) -> bool {
        self.is_shown()
            && self.stopped.is_none()
            && self.window.pointer_enabled()
            && !self.window.has_pending_click()
            && !self.window.has_pending_edit()
            && self.deferred_click.is_none()
            && self.javascript_input_idle()
            && self.lifecycle.owner().is_some_and(Owner::confirmed_active)
    }
    pub(crate) fn wake_at_ms(&self) -> u64 {
        let animation = if self.javascript_animation_allowed() {
            if self.javascript_resize_dirty() {
                0
            } else {
                self.javascript
                    .as_ref()
                    .and_then(|js| js.animation_due.into_iter().chain(js.timer_due).min())
                    .unwrap_or(u64::MAX)
            }
        } else {
            u64::MAX
        };
        self.expires_at_ms().min(animation)
    }
    /// Called only by sync_input after current application policy is established; refresh/begin_batch cannot seize the single slot.
    pub(in crate::desktop::surface_authorized_window) fn schedule_javascript_animation(
        &mut self,
        now: u64,
    ) -> Result<bool> {
        if !self.javascript_animation_allowed()
            || (!self.javascript_resize_dirty()
                && self
                    .javascript
                    .as_ref()
                    .and_then(|js| js.animation_due.into_iter().chain(js.timer_due).min())
                    .is_none_or(|due| due > now))
        {
            return Ok(false);
        }
        let current = self.javascript_mount()?;
        let animating = self.javascript_resize_dirty()
            || self
                .javascript
                .as_ref()
                .unwrap()
                .animation_due
                .is_some_and(|due| due <= now);
        self.window.set_submission_enabled(false)?;
        let viewport = self.capture_javascript_viewport()?;
        let js = self.javascript.as_mut().unwrap();
        let runtime = js.runtime.as_mut().unwrap();
        if animating {
            runtime.animation_frame(current, viewport)
        } else {
            runtime.timer_tick(current, viewport)
        }
        .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        if animating {
            js.animation_due = None;
        }
        js.timer_due = None;
        Ok(true)
    }
    pub(super) fn accept_javascript_animation(
        &mut self,
        pending: bool,
        observed: bool,
        timer_delay_ms: Option<u32>,
    ) -> Result<()> {
        // The child provides monotonic Instant timing; this wall clock only integrates with original application authorization-wait deadlines.
        let now = crate::desktop::surface_windows::now_ms()?;
        let js = self.javascript.as_mut().unwrap();
        // timer-only tasks must not continually postpone existing rAF deadlines, preventing high-frequency timers from starving painting.
        js.animation_due =
            pending.then(|| js.animation_due.unwrap_or_else(|| now.saturating_add(16)));
        js.timer_due = timer_delay_ms.map(|delay| now.saturating_add(u64::from(delay)));
        js.resize_observed = observed;
        js.observed_viewport_epoch = js.submitted_viewport_epoch;
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn javascript_animation_queued_for_test(&self) -> bool {
        self.javascript
            .as_ref()
            .is_some_and(|js| js.animation_due.is_some())
    }
    #[cfg(test)]
    pub(crate) fn javascript_timer_queued_for_test(&self) -> bool {
        self.javascript
            .as_ref()
            .is_some_and(|js| js.timer_due.is_some())
    }
}
