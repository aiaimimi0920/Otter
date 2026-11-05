//! 呈现快照与系统 EDIT 的接缝；只消费已经通过上层证据门禁的场景。
use super::super::surface_window::Activation;
use super::*;
use crate::{surface_click_event::ClickEvent, surface_live_binding::Binding};
impl Host {
    pub(crate) fn toggle_switch(&mut self, id: &str) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let visible = self
            .presentation
            .geometry()
            .nodes()
            .iter()
            .any(|node| node.id() == id && !node.clip().empty());
        Ok(visible && self.edits.toggle_switch(id)?)
    }
    pub(crate) fn switch_value(&self, id: &str) -> Option<bool> {
        self.edits.switch_value(id)
    }
    pub(crate) fn number_editable(&self, hwnd: HWND) -> bool {
        self.edits.number_editable(hwnd)
    }
    pub(crate) fn spinner_edit(&self, hwnd: HWND) -> Option<HWND> {
        self.edits.spinner_edit(hwnd)
    }
    pub(crate) fn cancel_spinners(&self) {
        self.edits.cancel_spinners();
        self.edits.cancel_sliders();
    }
    pub(crate) fn click_contract(
        &mut self,
        click: &super::super::surface_window::Click,
    ) -> Result<Vec<[u8; 32]>> {
        if !self.sync_viewport()? {
            return Err(Error::new(E_FAIL, "Surface viewport unavailable"));
        }
        if (click.source != Activation::Pointer && !self.presentation.focusable(&click.node_id))
            || (click.source == Activation::CapturedPointer
                && self.edits.slider_readonly(&click.node_id) != Some(false))
            || (click.source == Activation::Pointer
                && self.hit_test(click.client_pixels)? != Some(click.node_id.as_str()))
        {
            return Err(Error::new(E_FAIL, "Surface deferred target changed"));
        }
        crate::surface_click_route::ClickRoute::new(self.presentation.scene(), Some(&click.node_id))
            .map(|action| {
                crate::surface_edit_contract::full_fingerprint(action.node)
                    .map_err(|_| Error::new(E_INVALIDARG, "Surface click contract encoding failed"))
            })
            .collect()
    }
    pub(crate) fn input_fence(&mut self) -> super::super::surface_edit_mount::Fence {
        self.edits.seal_input_edge()
    }
    pub(crate) fn fence_state(
        &self,
        fence: &super::super::surface_edit_mount::Fence,
    ) -> super::super::surface_edit_mount::FenceState {
        self.edits.fence_state(fence)
    }
    #[cfg(test)]
    pub(crate) fn prepare_edit_at(
        &mut self,
        binding: &Binding,
        revision: u64,
        now: std::time::Instant,
    ) -> Result<Vec<ClickEvent>> {
        self.sync_viewport()?;
        self.edits
            .prepare_at(&self.presentation, binding, revision, now)
    }
    pub(crate) fn capture_form_values(&mut self) -> serde_json::Map<String, serde_json::Value> {
        self.edits.seal_input_edge();
        self.edits.values().clone()
    }
    /// 候选固定用户激活边沿的值；布局与动作资格仍按当前已验证场景复核。
    pub(crate) fn candidate_events(
        &mut self,
        click: &super::super::surface_window::Click,
        binding: &Binding,
        revision: u64,
    ) -> Result<Vec<ClickEvent>> {
        if !self.sync_viewport()? {
            return Ok(Vec::new());
        }
        if click.source == Activation::CapturedPointer
            && self.edits.slider_readonly(&click.node_id) != Some(false)
        {
            return Ok(Vec::new());
        }
        let events = if click.source != Activation::Pointer {
            self.presentation.keyboard_events_with_values(
                &click.node_id,
                binding,
                revision,
                click.modifiers,
                &click.values,
            )
        } else {
            let scale = 96. / self.viewport.1 as f32;
            self.presentation.click_events_with_values(
                click.client_pixels.map(|v| v as f32 * scale),
                binding,
                revision,
                click.modifiers,
                &click.values,
            )
        };
        events.map_err(|_| Error::new(E_INVALIDARG, "Surface candidate preparation failed"))
    }
    pub(super) fn sync_edits(&mut self) -> Result<()> {
        self.edits
            .sync(self.hwnd, &self.presentation, self.viewport.1)
    }
    pub(crate) fn bind_edits(&mut self, binding: &Binding) -> Result<()> {
        self.edits.bind(binding);
        self.sync_edits()
    }
    pub(crate) fn edit_policy(&mut self, editing: bool) -> Result<()> {
        self.edits.policy(editing)
    }
    pub(crate) fn edit_message(&mut self, hwnd: HWND, message: u32, code: u32) -> Result<()> {
        self.edits.notify(hwnd, message, code)
    }
    pub(crate) fn edit_pending(&self) -> bool {
        self.edits.pending()
    }
    pub(in crate::desktop) fn session_end_idle(&self) -> bool {
        self.edits.session_end_idle()
    }
    pub(crate) fn prepare_edit(
        &mut self,
        binding: &Binding,
        revision: u64,
    ) -> Result<Vec<ClickEvent>> {
        self.sync_viewport()?;
        self.edits.prepare(&self.presentation, binding, revision)
    }
    pub(crate) fn prepare_edit_before(
        &mut self,
        binding: &Binding,
        revision: u64,
        fence: &super::super::surface_edit_mount::Fence,
    ) -> Result<Vec<ClickEvent>> {
        self.sync_viewport()?;
        self.edits
            .prepare_before(&self.presentation, binding, revision, fence)
    }
    pub(crate) fn cancel_edits(&mut self, events: &[ClickEvent]) {
        self.edits.cancel(events);
    }
    pub(crate) fn prepare_javascript_edit(
        &mut self,
        binding: &Binding,
        fence: Option<&super::super::surface_edit_mount::Fence>,
        viewport_epoch: u64,
        pointer_active: bool,
        hover_active: bool,
    ) -> Result<Option<super::super::surface_edit_mount::JavaScriptInput>> {
        if !self.sync_viewport()? {
            return Err(Error::new(E_FAIL, "JavaScript edit viewport unavailable"));
        }
        self.edits.prepare_javascript(
            &self.presentation,
            binding,
            fence,
            viewport_epoch,
            pointer_active,
            hover_active,
        )
    }
    pub(crate) fn settle_javascript_edit(&mut self, id: &str) -> Result<()> {
        self.edits.settle_javascript(id)
    }
    pub(crate) fn javascript_enter(&mut self, hwnd: HWND, flags: u32, epoch: u64) -> Result<()> {
        self.edits.javascript_enter(hwnd, flags, epoch)
    }
    pub(crate) fn complete_javascript_edit(
        &mut self,
        id: &str,
        allowed: Option<bool>,
        focus: Option<(HWND, u64)>,
    ) -> Result<()> {
        self.edits.complete_javascript(id, allowed, focus)
    }
    pub(crate) fn fail_edits(&mut self, events: &[ClickEvent]) {
        self.edits.fail(events);
    }
    pub(crate) fn settle_edits(&mut self, events: &[ClickEvent]) -> Result<()> {
        self.edits.settle(events)
    }
    pub(crate) fn edit_owns(&self, hwnd: HWND) -> bool {
        self.edits.owns(hwnd)
    }
    pub(crate) fn focus_edit(&mut self, id: &str) -> Result<bool> {
        self.edits.focus(id)
    }
    pub(crate) fn navigate_edit(
        &mut self,
        hwnd: HWND,
        reverse: bool,
    ) -> Result<(Option<String>, bool)> {
        let current = self.edits.id(hwnd).map(str::to_owned);
        let Some(current) = current else {
            return Ok((None, false));
        };
        self.edits.notify(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::WM_COMMAND,
            windows::Win32::UI::WindowsAndMessaging::EN_KILLFOCUS,
        )?;
        let next = self.next_focus(Some(&current), reverse)?;
        let child = if let Some(id) = &next {
            self.edits.focus(id)?
        } else {
            false
        };
        Ok((next, child))
    }
}
