//! HWND left-click candidates; disabled by default, single-slot queued, with no actions/network requests in window callbacks.
use super::*;
use std::rc::Rc;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "surface_window_slider_click.rs"]
pub(super) mod slider_click;
#[path = "surface_window_text_selection.rs"]
mod text_selection;
pub(super) use text_selection::autoscroll::timer as text_drag_timer;
pub(super) use text_selection::copy_message;
pub(super) use text_selection::text_key_message;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Activation {
    Pointer,
    /// Native-control capture gestures retain the pressed target; release coordinates cannot redirect to another node.
    CapturedPointer,
    Keyboard,
    Automation,
}
pub struct Click {
    pub client_pixels: [i32; 2],
    pub node_id: String,
    pub modifiers: crate::surface_click_event::Modifiers,
    scene: Rc<()>,
    pub(crate) source: Activation,
    pub(crate) values: serde_json::Map<String, serde_json::Value>,
    pub(crate) fence: crate::desktop::surface_edit_mount::Fence,
    pub(crate) captured_at: std::time::Instant,
}
struct Press {
    point: [i32; 2],
    node: String,
}
#[derive(Default)]
pub(super) struct Pointer {
    selection_owner: Option<Rc<()>>,
    selection_scroll: Option<text_selection::autoscroll::Timer>,
    selection_cancel: u8,
    enabled: bool,
    hover: Option<String>,
    tracking: bool,
    pressed: Option<Press>,
    native_press: Option<slider_click::Press>,
    pending: Option<Click>,
    scene: Rc<()>,
}
pub(super) fn reconcile_selection(state: &State, host: &mut Host) {
    text_selection::reconcile(state, host);
}
impl Pointer {
    pub(super) fn identity(&self) -> Rc<()> {
        self.scene.clone()
    }
    pub(super) fn same_identity(&self, identity: &Rc<()>) -> bool {
        Rc::ptr_eq(&self.scene, identity)
    }
    pub(super) fn automation_busy(&self) -> bool {
        self.pending.is_some() || self.pressed.is_some() || self.native_press.is_some()
    }
    pub(super) fn queue_automation(
        &mut self,
        node_id: String,
        values: serde_json::Map<String, serde_json::Value>,
        fence: crate::desktop::surface_edit_mount::Fence,
    ) -> bool {
        if !self.enabled || self.automation_busy() {
            return false;
        }
        self.queue_keyboard(node_id, Default::default(), values, fence);
        self.pending.as_mut().unwrap().source = Activation::Automation;
        true
    }
    pub(super) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn hovered(&self) -> Option<&str> {
        self.hover.as_deref()
    }
    pub(super) fn pressed_node(&self) -> Option<&str> {
        self.pressed
            .as_ref()
            .map(|press| press.node.as_str())
            .filter(|id| Some(*id) == self.hovered())
    }
    pub(super) fn queue_keyboard(
        &mut self,
        node_id: String,
        modifiers: crate::surface_click_event::Modifiers,
        values: serde_json::Map<String, serde_json::Value>,
        fence: crate::desktop::surface_edit_mount::Fence,
    ) {
        if self.enabled && self.pending.is_none() {
            self.pending = Some(Click {
                client_pixels: [0, 0],
                node_id,
                modifiers,
                scene: self.scene.clone(),
                source: Activation::Keyboard,
                values,
                fence,
                captured_at: std::time::Instant::now(),
            });
        }
    }
    pub(super) fn enabled(&self) -> bool {
        self.enabled
    }
    pub(super) fn matches(&self, click: &Click) -> bool {
        Rc::ptr_eq(&self.scene, &click.scene)
    }
    pub(super) fn clear(&mut self) {
        self.selection_owner = None;
        self.selection_scroll = None;
        self.hover = None;
        self.pressed = None;
        self.native_press = None;
        self.pending = None;
    }
}
impl SurfaceWindow {
    /// Recheck window/scene identity before/after candidate consumption; perform re-hit testing and event preparation in one Host borrow.
    pub fn click_events_for_candidate(
        &self,
        click: Click,
        binding: &crate::surface_live_binding::Binding,
        revision: u64,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        if !self.is_alive() || !self.state.pointer.borrow().matches(&click) {
            return Err(Error::new(E_FAIL, "Surface click candidate is stale"));
        }
        let mut events = Vec::new();
        let ran = self.state.run(|host| {
            if click.source != Activation::Pointer {
                if click.source == Activation::Keyboard
                    && self.state.keyboard.borrow().focused() != Some(click.node_id.as_str())
                {
                    return Err(Error::new(E_FAIL, "Surface keyboard focus changed"));
                }
                events = host.candidate_events(&click, binding, revision)?;
                return Ok(());
            }
            if host.hit_test(click.client_pixels)? != Some(click.node_id.as_str()) {
                return Err(Error::new(E_FAIL, "Surface click target changed"));
            }
            events = host.candidate_events(&click, binding, revision)?;
            Ok(())
        })?;
        if !ran || !self.is_alive() || !self.state.pointer.borrow().matches(&click) {
            return Err(Error::new(E_FAIL, "Surface click candidate is stale"));
        }
        Ok(events)
    }
    /// Control local candidate capture only, granting no action permission; unchanged state preserves valid presses/candidates.
    pub fn set_pointer_enabled(&self, enabled: bool) -> Result<()> {
        self.set_input_policy(enabled, enabled)
    }
    /// Authorize editing and submission separately; soft-stopping submission revokes candidates without clearing logical focus or future edit owners.
    pub fn set_input_policy(&self, editing: bool, submitting: bool) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Surface window closed"));
        }
        let state = &self.state;
        let submitting = editing && submitting;
        let was_editing = state.local_edit.get();
        let was_submitting = state.pointer.borrow().enabled;
        if !state.input_policy_dirty.get() && was_editing == editing && was_submitting == submitting
        {
            return Ok(());
        }
        let revision = state
            .input_policy_revision
            .get()
            .checked_add(1)
            .expect("Surface input policy revision exhausted");
        state.input_policy_revision.set(revision);
        state.input_policy_dirty.set(true);
        if !editing {
            text_selection::cancel(state, true);
        }
        if !editing || !was_editing {
            invalidate(self.hwnd, state);
        } else {
            invalidate_soft(self.hwnd, state);
        }
        let current = || self.is_alive() && state.input_policy_revision.get() == revision;
        if !current() {
            return Err(Error::new(E_FAIL, "Surface edit policy superseded"));
        }
        // Revocation does not wait for Host; new authorization waits for successful native-control synchronization without synchronous-message generation changes.
        state.local_edit.set(was_editing && editing);
        state.pointer.borrow_mut().enabled = was_submitting && submitting;
        if !state.run(|host| {
            let result = host.edit_policy(editing);
            if editing && result.is_err() && current() && !state.local_edit.get() {
                // Failed authorization must not leave partially enabled native controls.
                result.and(host.edit_policy(false))
            } else {
                result
            }
        })? {
            return Err(Error::new(E_FAIL, "Surface edit policy unavailable"));
        }
        if !current() {
            return Err(Error::new(E_FAIL, "Surface edit policy superseded"));
        }
        state.local_edit.set(editing);
        state.pointer.borrow_mut().enabled = submitting;
        state.input_policy_dirty.set(false);
        Ok(())
    }
    pub fn set_submission_enabled(&self, enabled: bool) -> Result<()> {
        self.set_input_policy(self.state.local_edit.get(), enabled)
    }
    pub fn local_edit_enabled(&self) -> bool {
        self.is_alive() && self.state.local_edit.get()
    }
    pub fn take_click(&self) -> Option<Click> {
        if !self.is_alive() {
            return None;
        }
        self.state.pointer.borrow_mut().pending.take()
    }
    pub fn has_pending_click(&self) -> bool {
        self.is_alive() && self.state.pointer.borrow().pending.is_some()
    }
    pub fn pointer_enabled(&self) -> bool {
        self.is_alive() && self.state.pointer.borrow().enabled
    }
}
pub(super) fn cancel(hwnd: HWND, state: &State) {
    javascript_pointer::cancel(state);
    text_selection::cancel(state, false);
    gesture::clear(state);
    scrollbar::clear(state);
    state.pointer.borrow_mut().clear();
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        if GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
    }
}
/// Only an acquired Host's synchronous snapshot may queue; failure cannot substitute an empty form or retain borrows across callbacks.
pub(super) fn form_snapshot(
    state: &State,
) -> Option<(
    serde_json::Map<String, serde_json::Value>,
    crate::desktop::surface_edit_mount::Fence,
)> {
    let mut values = None;
    match state.run(|host| {
        values = Some((host.capture_form_values(), host.input_fence()));
        Ok(())
    }) {
        Ok(true) if !state.closed.get() => values,
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            None
        }
        _ => None,
    }
}
/// Scene identity also invalidates claimed but unprepared candidates, not merely the window's single slot.
pub(super) fn invalidate(hwnd: HWND, state: &State) {
    invalidate_candidates(hwnd, state);
    state.keyboard.borrow_mut().clear();
    focus_notifications::schedule(state);
}
pub(super) fn invalidate_candidates(hwnd: HWND, state: &State) {
    cancel(hwnd, state);
    state.keyboard.borrow_mut().cancel_press();
    state.pointer.borrow_mut().scene = Rc::new(());
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
/// Only for JS backpressure/same-Canvas frames/the next Canvas gesture on the same HWND; scrolling, focus changes, and revocation remain nonexempt.
pub(super) fn invalidate_soft(hwnd: HWND, state: &State) {
    if unsafe { javascript_pointer::preservable(hwnd, state) } {
        state.pointer.borrow_mut().clear();
        state.pointer.borrow_mut().scene = Rc::new(());
        state.keyboard.borrow_mut().cancel_press();
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    } else {
        invalidate_candidates(hwnd, state);
    }
}
fn point(lp: LPARAM) -> [i32; 2] {
    [
        (lp.0 as u16 as i16) as i32,
        ((lp.0 as usize >> 16) as u16 as i16) as i32,
    ]
}
fn hit(state: &State, point: [i32; 2]) -> Option<String> {
    let mut target = None;
    match state.run(|host| {
        target = host.hit_test(point)?.map(str::to_owned);
        Ok(())
    }) {
        Ok(true) if !state.closed.get() => target,
        Ok(_) => None,
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            None
        }
    }
}
pub(super) unsafe fn modifiers() -> crate::surface_click_event::Modifiers {
    crate::surface_click_event::Modifiers {
        alt_key: GetKeyState(VK_MENU.0 as i32) < 0,
        ctrl_key: GetKeyState(VK_CONTROL.0 as i32) < 0,
        meta_key: GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0,
        shift_key: GetKeyState(VK_SHIFT.0 as i32) < 0,
    }
}
unsafe fn dragged(press: &Press, point: [i32; 2]) -> bool {
    let limits = [
        GetSystemMetrics(SM_CXDRAG).max(1),
        GetSystemMetrics(SM_CYDRAG).max(1),
    ];
    (0..2).any(|axis| {
        (i64::from(point[axis]) - i64::from(press.point[axis])).abs() > i64::from(limits[axis])
    })
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, message: u32, lp: LPARAM) -> bool {
    // Suppress only this normal release's capturechanged; hiding/focus loss/resize on the same synchronous stack still hard-cancels.
    if message == WM_CAPTURECHANGED && state.pointer_releasing.get() {
        return false;
    }
    if message == WM_MOUSELEAVE {
        let mut pointer = state.pointer.borrow_mut();
        pointer.tracking = false;
        if pointer.hover.take().is_some() {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        return true;
    }
    if matches!(message, WM_CAPTURECHANGED | WM_CANCELMODE | WM_KILLFOCUS) {
        if message == WM_CANCELMODE {
            text_selection::cancel(state, true);
        }
        if message == WM_CAPTURECHANGED {
            cancel(hwnd, state);
        } else {
            invalidate(hwnd, state);
        }
        return false;
    }
    if state.closed.get() {
        return false;
    }
    if text_selection::message(hwnd, state, message, lp) {
        return true;
    }
    if !state.pointer.borrow().enabled {
        // Switch local presses remain valid; other nodes establish only navigation anchors during soft stop.
        let local_switch = state.local_edit.get()
            && hit(state, point(lp)).is_some_and(|id| switch::is_switch(state, &id));
        if !local_switch && state.pointer.borrow().pressed.is_none() {
            if state.local_edit.get() && matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
                if let Some(node) = hit(state, point(lp)) {
                    keyboard::focus_node(hwnd, state, &node);
                }
                return true;
            }
            return false;
        }
    }
    let point = point(lp);
    match message {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            // Do not overwrite unconsumed candidates; focus changes invalidate old ones, so preserve the original single-slot boundary first.
            if state.pointer.borrow().pending.is_some() {
                return true;
            }
            let Some(node) = hit(state, point) else {
                cancel(hwnd, state);
                return true;
            };
            keyboard::focus_node(hwnd, state, &node);
            state.pointer.borrow_mut().pressed = Some(Press { point, node });
            hover(hwnd, state, point);
            let _ = SetCapture(hwnd);
            if GetCapture() != hwnd {
                cancel(hwnd, state);
            }
        }
        WM_MOUSEMOVE => {
            let dragged = state
                .pointer
                .borrow()
                .pressed
                .as_ref()
                .is_some_and(|press| dragged(press, point));
            if dragged {
                cancel(hwnd, state);
            }
            hover(hwnd, state, point);
        }
        WM_LBUTTONUP => {
            let press = state.pointer.borrow_mut().pressed.take();
            let Some(press) = press else {
                return true;
            };
            let same = !dragged(&press, point)
                && GetCapture() == hwnd
                && hit(state, point).as_deref() == Some(press.node.as_str());
            let captured = GetCapture() == hwnd;
            let scene = state.pointer.borrow().scene.clone();
            // ReleaseCapture may synchronously reenter; clean capture before deciding whether to deliver candidates.
            if captured {
                let _ = ReleaseCapture();
            }
            if !Rc::ptr_eq(&scene, &state.pointer.borrow().scene) {
                return true;
            }
            if same && captured && !state.closed.get() && state.local_edit.get() {
                // Seal the prefix first: ancestor click belongs to the activation edge and must precede this Switch change.
                let Some((values, fence)) = form_snapshot(state) else {
                    return true;
                };
                let toggled = switch::activate(hwnd, state, &press.node);
                if toggled == Some(false)
                    || (toggled == Some(true) && !switch::has_click(state, &press.node))
                {
                    hover(hwnd, state, point);
                    return true;
                }
                if !state.pointer.borrow().enabled {
                    hover(hwnd, state, point);
                    return true;
                }
                let scene = state.pointer.borrow().scene.clone();
                state.pointer.borrow_mut().pending = Some(Click {
                    client_pixels: point,
                    node_id: press.node,
                    modifiers: modifiers(),
                    scene,
                    source: Activation::Pointer,
                    values,
                    fence,
                    captured_at: std::time::Instant::now(),
                });
            }
            hover(hwnd, state, point);
        }
        _ => return false,
    }
    true
}

unsafe fn hover(hwnd: HWND, state: &State, point: [i32; 2]) {
    let mut node = None;
    let result = state.run(|host| {
        let hit = host.hit_test(point)?.map(str::to_owned);
        if let Some(id) = hit {
            if host.focusable(&id)? {
                node = Some(id);
            }
        }
        Ok(())
    });
    if !matches!(result, Ok(true)) || state.closed.get() {
        return;
    }
    let mut pointer = state.pointer.borrow_mut();
    if pointer.hover != node {
        pointer.hover = node;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    drop(pointer);
    track_leave(hwnd, state);
}
pub(super) unsafe fn track_leave(hwnd: HWND, state: &State) {
    if !state.pointer.borrow().tracking {
        let mut track = TRACKMOUSEEVENT {
            cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        if TrackMouseEvent(&mut track).is_ok() {
            state.pointer.borrow_mut().tracking = true;
        }
    }
}
