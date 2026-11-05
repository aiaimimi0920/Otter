//! Canvas 左键捕获 owner；只采集原生消息，所有边沿进入原 Mount FIFO，不同步等待 child。
use super::*;
use crate::surface_javascript_wire::{PointerInput, PointerPhase};
#[path = "surface_window_javascript_hover.rs"]
mod hover;
pub(super) use hover::message as hover_message;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, GetFocus, IsWindowEnabled, ReleaseCapture, SetCapture,
};
#[derive(Default)]
pub(super) struct Pointer {
    sequence: u64,
    active: Option<Press>,
}
#[derive(Clone)]
struct Press {
    input: PointerInput,
    start: [i32; 2],
    gesture: u64,
    epoch: u64,
    dragged: bool,
}
pub(super) unsafe fn eligible(hwnd: HWND, state: &State) -> bool {
    hover_eligible(hwnd, state) && GetFocus() == hwnd
}
pub(super) unsafe fn hover_eligible(hwnd: HWND, state: &State) -> bool {
    state.javascript_keyboard.get()
        && !state.closed.get()
        && state.local_edit.get()
        && IsWindowVisible(hwnd).as_bool()
        && IsWindowEnabled(hwnd).as_bool()
        && !IsIconic(hwnd).as_bool()
        && GetForegroundWindow() == hwnd
}
fn queue(state: &State, press: &Press) -> bool {
    match state.run(|host| host.javascript_pointer(press.input.clone(), press.gesture, press.epoch))
    {
        Ok(true) if !state.closed.get() => true,
        result => {
            *state.error.borrow_mut() = Some(
                result
                    .err()
                    .unwrap_or_else(|| Error::new(E_FAIL, "Canvas pointer host unavailable")),
            );
            false
        }
    }
}
fn target(state: &State, point: [i32; 2]) -> Option<String> {
    let mut target = None;
    match state.run(|host| {
        target = host.canvas_pointer_target(point)?;
        Ok(())
    }) {
        Ok(true) if !state.closed.get() => target,
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            None
        }
        _ => None,
    }
}
pub(super) fn clear(state: &State) {
    state.javascript_pointer.borrow_mut().active = None;
}
pub(super) fn cancel(state: &State) {
    if !state.javascript_keyboard.get() {
        return;
    }
    clear(state);
    // 捕获结束不等于 FIFO 已交付；连已排 up 的 waiting 也要撤销，在途证据仍保留。
    if !state.closed.get() && state.local_edit.get() {
        match state.run(|host| host.cancel_javascript_pointers(state.viewport_epoch.get())) {
            Ok(false) => state.pointer_cancel_pending.set(true),
            Err(error) => {
                *state.error.borrow_mut() = Some(error);
            }
            _ => {}
        }
    }
}
unsafe fn release(hwnd: HWND, state: &State) {
    state.pointer_releasing.set(true);
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
    state.pointer_releasing.set(false);
}
/// 仅供软背压/JS 同节点帧/同 HWND 下一 Canvas 手势；其他 invalidate 都是硬取消。
pub(super) unsafe fn preservable(hwnd: HWND, state: &State) -> bool {
    let press = state.javascript_pointer.borrow().active.clone();
    let Some(press) = press else {
        return hover_eligible(hwnd, state) && GetCapture() != hwnd;
    };
    if !eligible(hwnd, state) || GetCapture() != hwnd || press.epoch != state.viewport_epoch.get() {
        return false;
    }
    let mut current = false;
    matches!(
        state.run(|host| {
            current = host.canvas_pointer_current(&press.input.target);
            Ok(())
        }),
        Ok(true)
    ) && current
        && !state.closed.get()
}
fn point(lp: LPARAM) -> [i32; 2] {
    [
        (lp.0 as u16 as i16) as i32,
        ((lp.0 as usize >> 16) as u16 as i16) as i32,
    ]
}
unsafe fn sample(hwnd: HWND, input: &mut PointerInput, pixels: [i32; 2]) {
    (input.client, input.modifiers) = data(hwnd, pixels);
}
unsafe fn data(hwnd: HWND, pixels: [i32; 2]) -> ([f64; 2], u8) {
    let scale = 96. / f64::from(windows::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(1));
    let client = pixels.map(|v| f64::from(v) * scale);
    let modifiers = pointer::modifiers();
    let modifiers = u8::from(modifiers.ctrl_key)
        | (u8::from(modifiers.shift_key) << 1)
        | (u8::from(modifiers.alt_key) << 2)
        | (u8::from(modifiers.meta_key) << 3);
    (client, modifiers)
}
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if !state.javascript_keyboard.get() {
        return false;
    }
    if matches!(message, WM_CAPTURECHANGED | WM_CANCELMODE | WM_KILLFOCUS) {
        return false;
    }
    let active = state.javascript_pointer.borrow().active.is_some();
    if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
        if active {
            return true;
        }
        let pixels = point(lp);
        let Some(node) = target(state, pixels) else {
            return false;
        };
        // 未获准的 Canvas 消息也不落回旧 click，不能在后台补出激活。
        if !state.local_edit.get() || !state.pointer.borrow().enabled() {
            return true;
        }
        keyboard::focus_canvas_node(hwnd, state, &node);
        if !eligible(hwnd, state) {
            return true;
        }
        let next = state.javascript_pointer.borrow().sequence.checked_add(1);
        let Some(gesture) = next else {
            return true;
        };
        let mut press = Press {
            input: PointerInput {
                target: node,
                phase: PointerPhase::Pointerdown,
                client: [0., 0.],
                modifiers: 0,
                click: false,
            },
            start: pixels,
            gesture,
            epoch: state.viewport_epoch.get(),
            dragged: false,
        };
        sample(hwnd, &mut press.input, pixels);
        {
            let mut owner = state.javascript_pointer.borrow_mut();
            owner.sequence = gesture;
            owner.active = Some(press.clone());
        }
        let _ = SetCapture(hwnd);
        let owns = state
            .javascript_pointer
            .borrow()
            .active
            .as_ref()
            .is_some_and(|p| p.gesture == gesture);
        if !owns {
            return true;
        } // 同 HWND 的后来 scrollbar/capture owner 不由本次释放。
        if !preservable(hwnd, state)
            || target(state, pixels).as_deref() != Some(press.input.target.as_str())
        {
            clear(state);
            release(hwnd, state);
            return true;
        }
        if !queue(state, &press) || !preservable(hwnd, state) {
            pointer::cancel(hwnd, state);
        }
        return true;
    }
    if !active || !matches!(message, WM_MOUSEMOVE | WM_LBUTTONUP) {
        return false;
    }
    if !preservable(hwnd, state) {
        pointer::cancel(hwnd, state);
        return true;
    }
    let mut press = state.javascript_pointer.borrow().active.clone().unwrap();
    let pixels = point(lp);
    let limits = [
        GetSystemMetrics(SM_CXDRAG).max(1),
        GetSystemMetrics(SM_CYDRAG).max(1),
    ];
    press.dragged |= (0..2)
        .any(|i| (i64::from(pixels[i]) - i64::from(press.start[i])).abs() > i64::from(limits[i]));
    sample(hwnd, &mut press.input, pixels);
    if message == WM_LBUTTONUP {
        press.input.phase = PointerPhase::Pointerup;
        press.input.click =
            !press.dragged && target(state, pixels).as_deref() == Some(press.input.target.as_str());
        clear(state);
        queue(state, &press);
        release(hwnd, state);
        hover_message(hwnd, state, message, wp, lp);
    } else {
        press.input.phase = PointerPhase::Pointermove;
        state.javascript_pointer.borrow_mut().active = Some(press.clone());
        if !queue(state, &press) {
            pointer::cancel(hwnd, state);
        }
    }
    true
}
