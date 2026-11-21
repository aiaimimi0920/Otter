//! 滚轮立即提交完整目标；timer 只负责空闲后的预览回收，不插值或排队追赶。
use super::{PinRect, State};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
const TIMER: usize = 0x4f5d;
const IDLE: Duration = Duration::from_millis(180);
#[derive(Clone, Copy)]
struct Motion {
    last: PinRect,
    received: Instant,
}
#[derive(Default)]
pub(in crate::desktop) struct Animation {
    motion: Cell<Option<Motion>>,
    moving: Cell<bool>,
    move_escape: RefCell<Option<super::move_escape::Guard>>,
    pub(super) parked: Cell<bool>,
    pub(super) refreshing: Cell<bool>,
    pub(super) preview: RefCell<Option<Rc<super::preview::Preview>>>,
    pub(super) click: Cell<Option<super::preview::Click>>,
}
pub(in crate::desktop) fn busy(state: &State) -> bool {
    state.pin_animation.moving.get()
        || state.pin_animation.motion.get().is_some()
        || (super::preview::active(state)
            && (!state.pin_animation.parked.get() || state.error.borrow().is_some()))
}
pub(in crate::desktop) fn moving(state: &State) -> bool {
    state.pin_animation.moving.get()
}
unsafe fn rect(hwnd: HWND) -> Option<PinRect> {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r).ok()?;
    Some(PinRect {
        x: r.left,
        y: r.top,
        width: (r.right - r.left).max(1) as u32,
        height: (r.bottom - r.top).max(1) as u32,
    })
}
pub unsafe fn wheel(hwnd: HWND, state: &State, wp: WPARAM, lp: LPARAM) {
    let delta = (wp.0 >> 16) as i16;
    if delta == 0 || !state.lifetime.owns(hwnd) {
        return;
    }
    let Some(current) = rect(hwnd) else {
        return;
    };
    if super::stationary::active(state) && !super::preview::fresh(state) {
        if let Err(error) = super::stationary::refresh(hwnd, state) {
            *state.error.borrow_mut() = Some(error);
            return;
        }
    }
    if !super::preview::fresh(state) {
        cancel(hwnd, state);
    }
    let pointer = (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32);
    let mut zoom = state.pin_zoom.get();
    let Some(mut target) = zoom.next(current, pointer, f32::from(delta) / 120.0) else {
        return;
    };
    // 只折叠同源同修饰键输入；自发的几何 ack 不构成用户输入屏障。
    // 不越过键盘、其他窗口或模式消息，不丢增量，不为过时尺寸各绘一帧。
    let proxy = super::preview::handle(state);
    for _ in 0..255 {
        let mut queued = MSG::default();
        if !PeekMessageW(&mut queued, None, 0, 0, PM_NOREMOVE).as_bool() {
            break;
        }
        if queued.hwnd == hwnd && queued.message == crate::desktop::window_geometry_wake::MESSAGE {
            // 此消息只清 pending 标记；应用仍在本次滚轮返回后结算，不在每个旧尺寸保存。
            if PeekMessageW(
                &mut queued,
                Some(hwnd),
                queued.message,
                queued.message,
                PM_REMOVE,
            )
            .as_bool()
            {
                DispatchMessageW(&queued);
            }
            continue;
        }
        if queued.message != WM_MOUSEWHEEL
            || (queued.hwnd != hwnd && Some(queued.hwnd) != proxy)
            || queued.wParam.0 as u16 != wp.0 as u16
            || !state.lifetime.owns(hwnd)
        {
            break;
        }
        let destination = queued.hwnd;
        if !PeekMessageW(
            &mut queued,
            Some(destination),
            WM_MOUSEWHEEL,
            WM_MOUSEWHEEL,
            PM_REMOVE,
        )
        .as_bool()
        {
            break;
        }
        let pointer = (
            queued.lParam.0 as i16 as i32,
            (queued.lParam.0 >> 16) as i16 as i32,
        );
        let delta = (queued.wParam.0 >> 16) as i16;
        if let Some(next) = zoom.next(target, pointer, f32::from(delta) / 120.0) {
            target = next;
        }
    }
    if !state.lifetime.owns(hwnd) {
        return;
    }
    state.pin_zoom.set(zoom);
    if target == current {
        return;
    }
    state.pin_animation.parked.set(false);
    if let Err(error) = super::preview::begin(hwnd, state, current) {
        state.pin_zoom.set(Default::default());
        *state.error.borrow_mut() = Some(error);
        return;
    }
    // 昂贵的软件绘制先完成；随后提交宿主几何和一直存在的共享白框。
    super::fit_size(hwnd, state, (target.width as i32, target.height as i32));
    if let Err(error) = super::preview::present(hwnd, state, target) {
        cancel(hwnd, state);
        *state.error.borrow_mut() = Some(error);
        return;
    }
    if !super::presentation::apply(hwnd, state, target) {
        cancel(hwnd, state);
        state.pin_zoom.set(Default::default());
        return;
    }
    state.pin_animation.motion.set(Some(Motion {
        last: target,
        received: Instant::now(),
    }));
    if SetTimer(Some(hwnd), TIMER, 32, None) == 0 {
        cancel(hwnd, state);
        *state.error.borrow_mut() = Some(windows::core::Error::from_win32());
    }
}
unsafe fn cancel(hwnd: HWND, state: &State) {
    state.pin_animation.parked.set(false);
    if state.pin_animation.motion.take().is_some() {
        let _ = KillTimer(Some(hwnd), TIMER);
    }
    super::preview::finish(hwnd, state, true);
}
pub(in crate::desktop) unsafe fn stop(hwnd: HWND, state: &State) {
    cancel(hwnd, state);
}
pub(in crate::desktop) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
) -> Option<LRESULT> {
    if msg == WM_ENTERSIZEMOVE {
        if !super::stationary::active(state) {
            cancel(hwnd, state);
        }
        state.pin_animation.parked.set(false);
        state.pin_animation.moving.set(true);
        if state.canvas.borrow().pinned && state.pin_animation.move_escape.borrow().is_none() {
            match super::move_escape::Guard::begin(hwnd) {
                Ok(guard) => *state.pin_animation.move_escape.borrow_mut() = Some(guard),
                Err(error) => *state.error.borrow_mut() = Some(error),
            }
        }
        super::movement::begin(hwnd, state);
    } else if msg == WM_EXITSIZEMOVE {
        state.pin_animation.moving.set(false);
        state.pin_animation.move_escape.borrow_mut().take();
        if !super::stationary::park(state) {
            cancel(hwnd, state);
        }
    } else if matches!(msg, WM_CANCELMODE | WM_DESTROY | WM_NCDESTROY) {
        state.pin_animation.moving.set(false);
    }
    if matches!(msg, WM_DESTROY | WM_NCDESTROY) {
        state.pin_animation.move_escape.borrow_mut().take();
        state.pin_animation.parked.set(false);
        super::preview::finish(hwnd, state, false);
    }
    if msg == WM_SHOWWINDOW && wp.0 == 0 && super::stationary::active(state) {
        cancel(hwnd, state);
    }
    if matches!(
        msg,
        WM_DESTROY
            | WM_NCDESTROY
            | WM_CANCELMODE
            | WM_LBUTTONDOWN
            | WM_NCLBUTTONDOWN
            | WM_CONTEXTMENU
    ) || (msg == WM_KEYDOWN && !matches!(wp.0, 0x11 | 0xa2 | 0xa3))
    {
        let continuing_pan = matches!(msg, WM_LBUTTONDOWN | WM_NCLBUTTONDOWN)
            && super::stationary::active(state)
            && super::stationary::eligible(state);
        if !continuing_pan {
            cancel(hwnd, state);
        }
    }
    if msg != WM_TIMER || wp.0 != TIMER {
        return None;
    }
    let Some(motion) = state.pin_animation.motion.get() else {
        return Some(LRESULT(0));
    };
    let blocked = state.group_locked.get()
        || state.group_hidden.get()
        || state.session_end.paused()
        || !IsWindowVisible(hwnd).as_bool()
        || !crate::desktop::capture_input::exclusive_available(state)
        || state.canvas.borrow().blocks_wheel();
    if blocked || rect(hwnd) != Some(motion.last) || !super::preview::fresh(state) {
        cancel(hwnd, state);
    } else if motion.received.elapsed() >= IDLE {
        state.pin_animation.motion.set(None);
        let _ = KillTimer(Some(hwnd), TIMER);
        if !super::stationary::park(state) {
            cancel(hwnd, state);
        }
    }
    Some(LRESULT(0))
}
#[cfg(test)]
pub(in crate::desktop) unsafe fn settle(hwnd: HWND, state: &State) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while state.pin_animation.motion.get().is_some() {
        assert!(Instant::now() < deadline, "zoom timer did not finish");
        let mut message = MSG::default();
        if PeekMessageW(&mut message, Some(hwnd), WM_TIMER, WM_TIMER, PM_REMOVE).as_bool() {
            DispatchMessageW(&message);
        } else {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
