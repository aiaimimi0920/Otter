//! 单槽本地滚动条捕获；复用Surface Repeat，Win32前移出owner，旧同步回调不能回挂新手势。
use super::*;
use crate::{desktop::surface_scroll_repeat::Repeat, tile_surface_scrollbar::Field};
use std::rc::Rc;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
enum Motion {
    Thumb(crate::surface_scroll::Drag),
    Page(Repeat),
}
struct Gesture {
    field: Field,
    motion: Motion,
}
#[derive(Default)]
pub(super) struct Owner {
    token: Rc<()>,
    gesture: Option<Gesture>,
    live: bool,
}
impl Owner {
    #[cfg(test)]
    pub(super) fn gesture_is_idle(&self) -> bool {
        !self.live && self.gesture.is_none()
    }
}
pub(super) fn captured(state: &State) -> bool {
    state.scrollbar.borrow().live
}
pub(super) fn cancel(hwnd: HWND, state: &State) {
    let (old, live) = {
        let mut owner = state.scrollbar.borrow_mut();
        let live = std::mem::take(&mut owner.live);
        owner.token = Rc::new(());
        (owner.gesture.take(), live)
    };
    drop(old);
    if live && unsafe { GetCapture() } == hwnd {
        unsafe {
            let _ = ReleaseCapture();
        }
    }
}
fn current(state: &State, field: &Field) -> Option<Field> {
    let frame = actions::active(state)?;
    state.renderer.borrow().as_ref()?.current_bar(&frame, field)
}
fn awaiting(state: &State, field: &Field) -> bool {
    !state.closed.get()
        && !state.changed.get()
        && field.awaiting(&state.shared.lock(), Instant::now())
}
fn owns(hwnd: HWND, state: &State) -> bool {
    !state.closed.get()
        && !state.changed.get()
        && unsafe { IsWindowVisible(hwnd).as_bool() && GetCapture() == hwnd }
}
fn same_token(state: &State, token: &Rc<()>) -> bool {
    let owner = state.scrollbar.borrow();
    owner.live && Rc::ptr_eq(&owner.token, token)
}
fn put(state: &State, token: &Rc<()>, gesture: Gesture) {
    if same_token(state, token) {
        state.scrollbar.borrow_mut().gesture = Some(gesture);
    }
}
fn apply(hwnd: HWND, state: &State, g: &Gesture, pixel: [i32; 2]) -> Result<()> {
    let Some(frame) = actions::active(state) else {
        return if awaiting(state, &g.field) {
            Ok(())
        } else {
            Err(Error::new(E_ABORT, "Tile scrollbar frame retired"))
        };
    };
    let field = state
        .renderer
        .borrow()
        .as_ref()
        .and_then(|r| r.current_bar(&frame, &g.field))
        .ok_or_else(|| Error::new(E_ABORT, "Tile scrollbar geometry changed"))?;
    let point = field
        .captured_point(&frame, pixel)
        .ok_or_else(|| Error::new(E_ABORT, "Tile scrollbar mapping changed"))?;
    let value = match &g.motion {
        Motion::Thumb(drag) => drag.value(point),
        Motion::Page(repeat) => {
            if field.bar.page_direction(point) != repeat.direction {
                return Ok(());
            }
            field.bar.page_value(point)
        }
    };
    let planned = state
        .renderer
        .borrow()
        .as_ref()
        .and_then(|r| r.plan_bar(&frame, &field, value));
    if let Some(scroll) = planned {
        scroll::install(
            hwnd,
            state,
            &field.instance,
            &scroll,
            &frame.arts[&field.instance].snapshot,
        )?;
    }
    Ok(())
}
fn begin(hwnd: HWND, state: &State, pixel: [i32; 2]) -> Result<bool> {
    let Some(frame) = actions::active(state) else {
        return Ok(false);
    };
    let field = state
        .renderer
        .borrow()
        .as_ref()
        .and_then(|r| r.scrollbar(&frame, pixel));
    let Some(field) = field else {
        cancel(hwnd, state);
        return Ok(false);
    };
    if !text::commit_for_click(state) {
        return Ok(true);
    }
    cancel(hwnd, state);
    actions::slider::cancel(hwnd, state);
    keyboard::close_for_text(state);
    keyboard::scrolled(state);
    if current(state, &field).is_none() || !unsafe { IsWindowVisible(hwnd).as_bool() } {
        return Ok(true);
    }
    let token = {
        let mut owner = state.scrollbar.borrow_mut();
        owner.token = Rc::new(());
        owner.live = true;
        owner.token.clone()
    };
    let motion = if let Some(drag) = field.bar.drag(field.point) {
        Motion::Thumb(drag)
    } else {
        let direction = field.bar.page_direction(field.point);
        if direction == 0 {
            cancel(hwnd, state);
            return Ok(true);
        }
        match Repeat::new(hwnd, field.bar.id.clone(), field.bar.axis, direction, pixel) {
            Ok(repeat) => Motion::Page(repeat),
            Err(error) => {
                if same_token(state, &token) {
                    cancel(hwnd, state);
                }
                return Err(error);
            }
        }
    };
    if !same_token(state, &token) {
        return Ok(true);
    }
    put(state, &token, Gesture { field, motion });
    unsafe {
        let _ = SetCapture(hwnd);
    }
    if same_token(state, &token) {
        if !owns(hwnd, state) {
            cancel(hwnd, state);
        } else {
            drive(hwnd, state, WM_MOUSEMOVE, pixel, 0, Instant::now(), true)?;
        }
    }
    Ok(true)
}
fn drive(
    hwnd: HWND,
    state: &State,
    msg: u32,
    pixel: [i32; 2],
    timer: usize,
    now: Instant,
    initial: bool,
) -> Result<bool> {
    let (token, gesture) = {
        let mut owner = state.scrollbar.borrow_mut();
        (owner.token.clone(), owner.gesture.take())
    };
    let Some(mut gesture) = gesture else {
        return Ok(false);
    };
    if msg == WM_TIMER && !matches!(&gesture.motion,Motion::Page(r) if r.id()==timer) {
        put(state, &token, gesture);
        return Ok(false);
    }
    if !owns(hwnd, state)
        || current(state, &gesture.field).is_none() && !awaiting(state, &gesture.field)
    {
        if same_token(state, &token) {
            cancel(hwnd, state);
        }
        return Ok(true);
    }
    let result = (|| {
        let point = match &mut gesture.motion {
            Motion::Thumb(_) => pixel,
            Motion::Page(repeat) => {
                if msg == WM_MOUSEMOVE {
                    repeat.point = pixel;
                }
                if !initial && (msg != WM_TIMER || !repeat.ready(timer, now)?) {
                    return Ok(());
                }
                repeat.point
            }
        };
        apply(hwnd, state, &gesture, point)
    })();
    if same_token(state, &token) {
        if result.is_err() || msg == WM_LBUTTONUP {
            cancel(hwnd, state);
        } else {
            put(state, &token, gesture);
        }
    }
    result.map(|_| true)
}
pub(super) fn tick(hwnd: HWND, state: &State) {
    let field = state
        .scrollbar
        .borrow()
        .gesture
        .as_ref()
        .map(|g| g.field.clone());
    if field
        .is_some_and(|f| !owns(hwnd, state) || current(state, &f).is_none() && !awaiting(state, &f))
    {
        cancel(hwnd, state);
    }
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    if matches!(
        msg,
        WM_CAPTURECHANGED
            | WM_CANCELMODE
            | WM_KILLFOCUS
            | WM_DISPLAYCHANGE
            | WM_DPICHANGED
            | WM_CLOSE
            | WM_DESTROY
    ) || msg == WM_SHOWWINDOW && wp.0 == 0
        || matches!(msg, WM_MOUSEWHEEL | WM_MOUSEHWHEEL)
    {
        cancel(hwnd, state);
        return false;
    }
    let point = [
        lp.0 as u16 as i16 as i32,
        (lp.0 as usize >> 16) as u16 as i16 as i32,
    ];
    let result = match msg {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => begin(hwnd, state, point),
        WM_MOUSEMOVE | WM_LBUTTONUP | WM_TIMER => {
            drive(hwnd, state, msg, point, wp.0, Instant::now(), false)
        }
        _ => return false,
    };
    match result {
        Ok(handled) => handled,
        // begin/drive只退休自身token；同步paint中接管的successor不能被外层错误清理。
        Err(error) => {
            state.shared.lock().actions.notice = "Art滚动条未完成";
            #[cfg(test)]
            eprintln!("tile-scrollbar: {error}");
            let _ = error;
            true
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_scrollbar_window_tests.rs"]
mod tests;
