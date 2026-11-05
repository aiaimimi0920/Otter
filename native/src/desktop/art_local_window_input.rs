//! 本地 Art 单槽输入与定时排程；复用 SurfaceWindow FIFO，未知交付失败终止而不重放。
use super::*;
use crate::{surface_javascript::signed::runtime::Operation, surface_viewport::Viewport};
use std::time::{Duration, Instant};
pub(super) struct Input {
    pub view: crate::surface_live_binding::Binding,
    pub deferred: Option<super::super::surface_window::DeferredClick>,
    pub ticket: Option<String>,
    pub viewport: Option<Viewport>,
    pub spatial: bool,
    pub epoch: u64,
    pub focus: u64,
    pub observed_epoch: u64,
    pub resize_observed: bool,
    pub animation: Option<Instant>,
    pub timer: Option<Instant>,
    active: bool,
    configured: bool,
    suspended: bool,
}
impl Input {
    pub fn new(view: crate::surface_live_binding::Binding) -> Self {
        Self {
            view,
            deferred: None,
            ticket: None,
            viewport: None,
            spatial: false,
            epoch: 0,
            focus: 0,
            observed_epoch: 0,
            resize_observed: false,
            animation: None,
            timer: None,
            active: false,
            configured: false,
            suspended: false,
        }
    }
    pub fn clear(&mut self) {
        self.deferred = None;
        self.ticket = None;
        self.animation = None;
        self.timer = None;
        self.active = false;
    }
    pub(super) fn schedule_at(
        &mut self,
        now: Instant,
        animation: bool,
        timer: Option<u32>,
        observed: bool,
    ) {
        // timer 回复不能推迟尚未消费的 rAF；派发 Animation 时才清除其期限。
        self.animation =
            animation.then(|| self.animation.unwrap_or(now + Duration::from_millis(16)));
        self.timer = timer.map(|delay| now + Duration::from_millis(u64::from(delay)));
        self.resize_observed = observed;
        self.observed_epoch = self.epoch;
    }
}
impl Window {
    pub fn deadline(&self) -> Option<Instant> {
        if self.stopping || !self.input.active || self.runtime.pending() || self.result.is_some() {
            return None;
        }
        let window = self.window.as_ref()?;
        if !window.pointer_enabled()
            || window.has_pending_click()
            || window.has_pending_edit()
            || self.input.deferred.is_some()
        {
            return None;
        }
        self.input
            .animation
            .into_iter()
            .chain(self.input.timer)
            .min()
    }
    pub(super) fn submit_input(
        &mut self,
        operation: Operation,
        spatial: bool,
        host: &impl Fn() -> bool,
    ) -> Result<()> {
        let window = self.window.as_ref().unwrap();
        self.input.viewport = Some(window.viewport()?);
        self.input.epoch = window.viewport_epoch();
        self.input.focus = window.javascript_focus_epoch();
        self.input.spatial = spatial;
        window.set_submission_enabled(false)?;
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art input host changed"));
        }
        self.runtime
            .submit(&self.current, operation)
            .map_err(|_| Error::new(E_FAIL, "Local Art input submission failed"))
    }
    pub(super) fn advance(&mut self, host: &impl Fn() -> bool, visible: bool) -> Result<()> {
        if !self.has_frame || self.result.is_some() {
            return Ok(());
        }
        let Some(window) = &self.window else {
            return Ok(());
        };
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art host changed"));
        }
        if let Some(error) = window.take_error() {
            return Err(error);
        }
        let hwnd = window.dialog_owner()?;
        let active = visible
            && unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
                IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool()
            };
        self.input.active = active;
        window.set_input_policy(active, active && !self.runtime.pending())?;
        if !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art input policy changed"));
        }
        if self.runtime.pending() {
            return Ok(());
        }
        if self.advance_trigger(host)? {
            return Ok(());
        }
        let window = self.window.as_ref().unwrap();
        if !active {
            self.input.deferred = None;
            if !self.input.suspended {
                self.input.suspended = true;
                self.submit_input(Operation::Suspend, false, host)?;
            }
            return Ok(());
        }
        let viewport = window.viewport()?;
        if self.input.suspended || !self.input.configured {
            let operation = if self.input.suspended {
                Operation::Viewport(viewport)
            } else {
                Operation::Animation(viewport)
            };
            self.input.suspended = false;
            self.input.configured = true;
            return self.submit_input(operation, false, host);
        }
        if self.input.deferred.is_none() {
            if let Some(click) = window.take_click() {
                self.input.deferred = Some(window.defer_click(click, &self.input.view, 1)?);
            }
        }
        if let Some(pending) = &self.input.deferred {
            use super::super::surface_edit_mount::FenceState;
            match window.deferred_state(pending) {
                FenceState::Invalid => {
                    return Err(Error::new(E_FAIL, "Local Art click dependency expired"))
                }
                FenceState::InFlight => return Ok(()),
                _ => {}
            }
        }
        if let Some(prepared) =
            window.prepare_javascript_edit(&self.input.view, self.input.deferred.as_ref())?
        {
            let spatial = matches!(
                prepared.input,
                crate::surface_javascript_wire::LocalInput::Pointer(_)
                    | crate::surface_javascript_wire::LocalInput::Hover(_)
            );
            self.input.ticket = Some(prepared.id);
            return self.submit_input(Operation::Input(prepared.input, viewport), spatial, host);
        }
        if let Some(pending) = &self.input.deferred {
            if window.deferred_state(pending) != super::super::surface_edit_mount::FenceState::Ready
            {
                return Ok(());
            }
            if !window
                .prepare_deferred(pending, &self.input.view, 1)?
                .is_empty()
            {
                return Err(Error::new(
                    E_FAIL,
                    "Local Art cannot submit remote declarative actions",
                ));
            }
            let target = pending.target().to_owned();
            self.input.deferred = None;
            return self.submit_input(Operation::ClickAt(target, viewport), false, host);
        }
        if window.has_pending_edit() {
            return Ok(());
        }
        let now = Instant::now();
        let animation = (self.input.resize_observed
            && self.input.observed_epoch != window.viewport_epoch())
            || self.input.animation.is_some_and(|due| due <= now);
        if animation || self.input.timer.is_some_and(|due| due <= now) {
            if animation {
                self.input.animation = None;
            }
            self.input.timer = None;
            self.submit_input(
                if animation {
                    Operation::Animation(viewport)
                } else {
                    Operation::Timer(viewport)
                },
                false,
                host,
            )?;
        }
        Ok(())
    }
    pub(super) fn schedule(&mut self, animation: bool, timer: Option<u32>, observed: bool) {
        self.input
            .schedule_at(Instant::now(), animation, timer, observed);
    }
}
