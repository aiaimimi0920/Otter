//! 无handler本地值接线；纯cache借用不跨Win32调用，焦点切换不丢值，隐藏/减权退休。
use super::*;
use crate::{tile_surface_focus::Focus, tile_surface_local::Value};
pub(super) fn focus(state: &State, pixel: [i32; 2]) -> Option<Focus> {
    let frame = actions::active(state)?;
    state.renderer.borrow().as_ref()?.focus_at(&frame, pixel)
}
pub(super) fn value(state: &State, focus: &Focus) -> Option<Value> {
    state.local.borrow_mut().value(focus)
}
pub(super) fn set(state: &State, focus: Focus, value: Value) -> Result<()> {
    // 只有实际paint且仍匹配合同的输入能修改cache；等待/旧HWND消息不能写本地状态。
    let current = keyboard::current(state, &focus)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Tile local field is no longer painted"))?;
    let counts = state
        .text
        .borrow()
        .as_ref()
        .map_or((0, 0), |o| o.counts(&current.anchor.view));
    state
        .local
        .borrow_mut()
        .set(current, value, counts)
        .map_err(|e| Error::new(E_OUTOFMEMORY, e))?;
    state.text_dirty.set(true);
    Ok(())
}
pub(super) fn clear(state: &State) {
    if state.local.borrow_mut().clear() {
        state.text_dirty.set(true);
    }
}
pub(super) fn tick(state: &State) {
    let blocked = {
        let s = state.shared.lock();
        s.stopped
            || s.expired
            || s.frozen
            || s.black
            || s.identification.blocked()
            || s.until.is_none_or(|v| Instant::now() >= v)
    };
    if blocked || state.closed.get() || state.changed.get() {
        clear(state);
        return;
    }
    let changed = state.local.borrow_mut().reconcile(
        |f| {
            actions::active(state)
                .and_then(|frame| state.renderer.borrow().as_ref()?.retained_focus(&frame, f))
        },
        |f| f.awaiting(&state.shared.lock(), Instant::now()),
    );
    if changed {
        state.text_dirty.set(true);
    }
}
#[cfg(test)]
#[path = "tile_surface_local_window_tests.rs"]
mod tests;
#[cfg(test)]
impl Window {
    pub(crate) fn test_art_local_values(&self) -> serde_json::Value {
        tick(&self.state);
        let values: serde_json::Map<_, _> = self
            .state
            .local
            .borrow()
            .snapshot()
            .iter()
            .map(|e| {
                let v = match e.value {
                    Value::Switch(v) => serde_json::json!(v),
                    Value::Select(v) => serde_json::json!(v),
                    Value::Slider(v) => serde_json::json!(v),
                };
                (e.focus.node.clone(), v)
            })
            .collect();
        values.into()
    }
    pub(crate) fn test_art_local_submit(&self) -> bool {
        let before = self.state.shared.lock().actions.test_offered;
        for _ in 0..6 {
            if self
                .state
                .keyboard
                .borrow()
                .focus
                .as_ref()
                .is_some_and(|f| f.kind == crate::surface_scene::Kind::Button)
            {
                break;
            }
            keyboard::navigate(self.hwnd, &self.state, false);
        }
        unsafe {
            SendMessageW(self.hwnd, WM_KEYDOWN, Some(WPARAM(13)), None);
        }
        self.state.shared.lock().actions.test_offered > before
    }
}
