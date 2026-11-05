//! 单槽依赖点击：保存激活值与原合同，只在前置编辑完整结算后重新核验并编码。
use super::*;
pub(crate) use crate::desktop::surface_edit_mount::FenceState;
use crate::{surface_click_event::ClickEvent, surface_live_binding::Binding};
use std::time::{Duration, Instant};
pub(crate) struct DeferredClick {
    click: Click,
    contract: Vec<[u8; 32]>,
    binding: Binding,
    revision: u64,
    expires: Instant,
}
impl DeferredClick {
    pub(crate) fn target(&self) -> &str {
        &self.click.node_id
    }
    pub(crate) fn fence(&self) -> &crate::desktop::surface_edit_mount::Fence {
        &self.click.fence
    }
}
impl SurfaceWindow {
    pub(crate) fn defer_click(
        &self,
        click: Click,
        binding: &Binding,
        revision: u64,
    ) -> Result<DeferredClick> {
        let expires = click.captured_at + Duration::from_secs(30);
        if Instant::now() >= expires {
            return Err(Error::new(E_FAIL, "Surface click dependency expired"));
        }
        if !self.is_alive() || !self.state.pointer.borrow().matches(&click) {
            return Err(Error::new(E_FAIL, "Surface click candidate is stale"));
        }
        if click.source == pointer::Activation::Keyboard
            && self.state.keyboard.borrow().focused() != Some(click.node_id.as_str())
        {
            return Err(Error::new(E_FAIL, "Surface keyboard focus changed"));
        }
        let mut contract = Vec::new();
        if !self.state.run(|host| {
            contract = host.click_contract(&click)?;
            Ok(())
        })? || !self.is_alive()
        {
            return Err(Error::new(E_FAIL, "Surface dependency capture unavailable"));
        }
        Ok(DeferredClick {
            click,
            contract,
            binding: binding.clone(),
            revision,
            expires,
        })
    }
    pub(crate) fn deferred_state(&self, pending: &DeferredClick) -> FenceState {
        self.deferred_state_at(pending, Instant::now())
    }
    pub(crate) fn deferred_state_at(&self, pending: &DeferredClick, now: Instant) -> FenceState {
        if !self.is_alive() || now >= pending.expires {
            return FenceState::Invalid;
        }
        self.state
            .host
            .borrow()
            .as_ref()
            .map_or(FenceState::Invalid, |host| {
                host.fence_state(&pending.click.fence)
            })
    }
    pub(crate) fn prepare_deferred(
        &self,
        pending: &DeferredClick,
        binding: &Binding,
        revision: u64,
    ) -> Result<Vec<ClickEvent>> {
        if self.deferred_state(pending) != FenceState::Ready
            || !self.pointer_enabled()
            || binding != &pending.binding
            || revision < pending.revision
        {
            return Err(Error::new(E_FAIL, "Surface deferred click inactive"));
        }
        let mut events = Vec::new();
        let ran = self.state.run(|host| {
            if host.click_contract(&pending.click)? != pending.contract {
                return Err(Error::new(E_FAIL, "Surface deferred contract changed"));
            }
            events = host.candidate_events(&pending.click, binding, revision)?;
            Ok(())
        })?;
        if !ran || !self.is_alive() {
            return Err(Error::new(
                E_FAIL,
                "Surface deferred preparation unavailable",
            ));
        }
        Ok(events)
    }
}
