//! Tile自有捕获手势；无子HWND/线程/hook，不抢焦点，Win32重入前释放所有RefCell/Shared借用。
use super::*;
use crate::tile_surface_slider::{Drag, Field};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
fn field(state: &State, pixel: [i32; 2]) -> Option<Field> {
    let frame = active(state)?;
    state.renderer.borrow().as_ref()?.slider(&frame, pixel)
}
pub(in crate::desktop::tile_window) fn current(state: &State, drag: &Drag) -> bool {
    drag.edit.active() && field(state, drag.field.anchor.pixel).is_some_and(|f| drag.field.same(&f))
}
fn awaiting(state: &State, drag: &Drag) -> bool {
    !state.closed.get()
        && !state.changed.get()
        && drag.edit.active()
        && drag
            .field
            .awaiting_frame(&state.shared.lock(), Instant::now())
}
pub(in crate::desktop::tile_window) fn retained(state: &State, drag: &Drag) -> bool {
    current(state, drag) || awaiting(state, drag)
}
pub(in crate::desktop::tile_window) fn waiting(state: &State) -> bool {
    match &*state.pressed.borrow() {
        Gesture::Slider(drag) => awaiting(state, drag),
        _ => state
            .slider_preview
            .borrow()
            .as_ref()
            .is_some_and(|d| awaiting(state, d)),
    }
}
pub(in crate::desktop::tile_window) fn cancel(hwnd: HWND, state: &State) {
    let prior = std::mem::take(&mut *state.pressed.borrow_mut());
    let preview = state.slider_preview.borrow_mut().take().is_some();
    let changed = !matches!(prior, Gesture::None) || preview;
    if let Gesture::Slider(drag) = prior {
        let mut s = state.shared.lock();
        #[cfg(test)]
        eprintln!(
            "tile-slider-cancel: active={} frame={} presented={} prepared={} frozen={} black={} status={} notice={}",
            drag.edit.active(), s.frame.is_some(), s.presented_frame.is_some(), s.art.is_some(),
            s.frozen, s.black, s.status, s.actions.notice
        );
        s.actions.cancel_edit(&drag.edit);
    }
    unsafe {
        if GetCapture() == hwnd && !super::super::scrollbar::captured(state) {
            let _ = ReleaseCapture();
        }
        if changed && !state.closed.get() {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}
pub(in crate::desktop::tile_window) fn settle(state: &State) -> bool {
    let stale = state.slider_preview.borrow().as_ref().is_some_and(|drag| {
        !retained(state, drag)
            || active(state).map_or_else(
                || !awaiting(state, drag),
                |frame| {
                    !state.shared.lock().actions.preview_pending(
                        &drag.edit,
                        &frame,
                        &drag.field.anchor.view.instance_id,
                    )
                },
            )
    });
    if stale {
        state.slider_preview.borrow_mut().take();
    }
    stale
}
pub(in crate::desktop::tile_window) fn begin(hwnd: HWND, state: &State, pixel: [i32; 2]) -> bool {
    let Some(field) = field(state, pixel) else {
        return false;
    };
    cancel(hwnd, state);
    if state.closed.get() || state.changed.get() {
        return true;
    }
    let mut drag = Drag::new(field);
    if let Some(crate::tile_surface_local::Value::Slider(v)) =
        super::super::local::focus(state, pixel)
            .and_then(|focus| super::super::local::value(state, &focus))
    {
        drag.local_start(v);
    }
    if let Some(edit) = super::super::text::peer(
        state,
        &drag.field.anchor.view,
        drag.edit.node.clone(),
        drag.edit.contract,
    ) {
        drag.edit = edit;
    }
    *state.pressed.borrow_mut() = Gesture::Slider(Box::new(drag));
    unsafe {
        SetCapture(hwnd);
    }
    if unsafe { GetCapture() } != hwnd {
        cancel(hwnd, state);
        return true;
    }
    update(hwnd, state, pixel);
    true
}
pub(in crate::desktop::tile_window) fn update(hwnd: HWND, state: &State, pixel: [i32; 2]) {
    let prior = std::mem::take(&mut *state.pressed.borrow_mut());
    let Gesture::Slider(mut drag) = prior else {
        *state.pressed.borrow_mut() = prior;
        return;
    };
    if unsafe { GetCapture() } == hwnd && awaiting(state, &drag) {
        // 尚无可操作的已paint画面；不取样/排命令，但不能把本手势的权威换值当取消。
        *state.pressed.borrow_mut() = Gesture::Slider(drag);
        return;
    }
    if unsafe { GetCapture() } != hwnd || !current(state, &drag) {
        state.shared.lock().actions.cancel_edit(&drag.edit);
        cancel(hwnd, state);
        return;
    }
    let Some(frame) = active(state) else {
        cancel(hwnd, state);
        return;
    };
    let result = drag.sample(&frame, pixel);
    let accepted = match result {
        Ok(Some(command)) => state
            .shared
            .lock()
            .actions
            .offer(vec![command], Instant::now()),
        Ok(None) => true,
        Err(error) => {
            state.shared.lock().actions.notice = error;
            false
        }
    };
    if accepted {
        *state.pressed.borrow_mut() = Gesture::Slider(drag);
    } else {
        state.shared.lock().actions.cancel_edit(&drag.edit);
        cancel(hwnd, state);
    }
    if !state.closed.get() {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}
pub(super) fn finish(hwnd: HWND, state: &State, mut drag: Drag, pixel: [i32; 2]) {
    let valid = unsafe { GetCapture() } == hwnd && current(state, &drag);
    // 先移出手势再ReleaseCapture；同步WM_CAPTURECHANGED不能取消正常抬起已形成的批次。
    unsafe {
        if GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
    }
    if !valid || !current(state, &drag) {
        let mut s = state.shared.lock();
        s.actions.cancel_edit(&drag.edit);
        s.actions.notice = "Art滑块画面已失效；未提交";
        return;
    }
    let Some(frame) = active(state) else {
        return;
    };
    let result = (|| {
        let mut commands = Vec::new();
        if let Some(command) = drag.sample(&frame, pixel)? {
            commands.push(command);
        }
        if let Some(command) = drag.commit()? {
            commands.push(command);
        }
        if let Some(focus) = super::super::local::focus(state, drag.field.anchor.pixel) {
            if matches!(
                super::super::local::value(state, &focus),
                Some(crate::tile_surface_local::Value::Slider(_))
            ) {
                super::super::local::set(
                    state,
                    focus,
                    crate::tile_surface_local::Value::Slider(drag.value),
                )
                .map_err(|_| "Art本地滑块未修改")?;
            }
        }
        // 同普通Slider，commit先于祖先click。离开原字段抬起只提交值，不合成外部click。
        if field(state, pixel).is_some_and(|f| drag.field.same(&f)) {
            for mut command in prepare(state, pixel) {
                command.edit = Some(drag.edit.clone());
                commands.push(command);
            }
        }
        Ok::<_, &'static str>(commands)
    })();
    match result {
        Ok(commands) if commands.is_empty() => {
            if state.shared.lock().actions.preview_pending(
                &drag.edit,
                &frame,
                &drag.field.anchor.view.instance_id,
            ) {
                *state.slider_preview.borrow_mut() = Some(drag);
            }
        }
        Ok(commands) => {
            let mut s = state.shared.lock();
            if !s.actions.offer(commands, Instant::now()) {
                s.actions.cancel_edit(&drag.edit);
            } else {
                *state.slider_preview.borrow_mut() = Some(drag);
            }
        }
        Err(error) => {
            let mut s = state.shared.lock();
            s.actions.cancel_edit(&drag.edit);
            s.actions.notice = error;
        }
    }
    unsafe {
        if !state.closed.get() {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_slider_window_tests.rs"]
mod tests;
