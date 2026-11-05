//! Shift 拖出手势及单槽准备任务；仅活动准备期间使用完成定时器，不持有跨 OLE 的 UI 借用。
use super::{drag_prepare, State};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[path = "drag_input_finish.rs"]
mod finish;
const TIMER: usize = 0x4F62;
#[derive(Default)]
pub(super) struct Session {
    origin: Cell<Option<(i32, i32)>>,
    pending: RefCell<Option<Pending>>,
    active: RefCell<Option<Arc<AtomicBool>>>,
}
struct Pending {
    job: drag_prepare::Job,
    stamp: super::display_snapshot::Stamp,
}

fn can_begin(state: &State) -> bool {
    // 编辑工具拥有 Shift（追加框选、形状约束等）；拖出仅在平移/浏览模式接管手势。
    state.canvas.borrow().tool == crate::editor::Tool::Pan
        && (state.canvas.borrow().image.is_some() || state.resolved_display.borrow().is_some())
}
fn point(lp: LPARAM) -> (i32, i32) {
    (lp.0 as i16 as i32, (lp.0 >> 16) as i16 as i32)
}
fn crossed(start: (i32, i32), current: (i32, i32), threshold: (i32, i32)) -> bool {
    start.0.abs_diff(current.0) >= threshold.0.max(1) as u32
        || start.1.abs_diff(current.1) >= threshold.1.max(1) as u32
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    let active = state.drag.active.borrow().clone();
    if let Some(cancel) = active {
        if matches!(msg, WM_CLOSE | WM_DESTROY | WM_CANCELMODE)
            || (msg == WM_KEYDOWN && wp.0 == VK_ESCAPE.0 as usize)
        {
            cancel.store(true, Ordering::Release);
        }
        // OLE 拥有手势；嵌套消息不能启动编辑或第二个导出/文件对话框。
        if matches!(
            msg,
            WM_LBUTTONDOWN
                | WM_LBUTTONUP
                | WM_LBUTTONDBLCLK
                | WM_MOUSEMOVE
                | WM_MOUSEWHEEL
                | WM_MOUSEHWHEEL
                | WM_KEYDOWN
                | WM_KEYUP
                | WM_COMMAND
                | WM_CONTEXTMENU
        ) {
            return Some(LRESULT(0));
        }
        return None;
    }
    let engaged = state.drag.origin.get().is_some()
        || state
            .drag
            .pending
            .borrow()
            .as_ref()
            .is_some_and(|pending| !pending.job.is_cancelled());
    match msg {
        WM_TIMER if wp.0 == TIMER => {
            finish::poll(hwnd, state, GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0);
        }
        WM_DESTROY => {
            dispose(hwnd, state);
            return None;
        }
        WM_CLOSE | WM_CANCELMODE | WM_KILLFOCUS | WM_CAPTURECHANGED => {
            cancel(hwnd, state);
            return None;
        }
        WM_KEYDOWN if engaged && wp.0 == VK_ESCAPE.0 as usize => cancel(hwnd, state),
        WM_LBUTTONUP if engaged => cancel(hwnd, state),
        WM_NCHITTEST
            if state.canvas.borrow().pinned
                && can_begin(state)
                && GetKeyState(VK_SHIFT.0 as i32) < 0 =>
        {
            return Some(LRESULT(HTCLIENT as isize))
        }
        WM_LBUTTONDOWN if GetKeyState(VK_SHIFT.0 as i32) < 0 && can_begin(state) => {
            if state.modals.blocked()
                || state.file_dialog.get()
                || super::group_lock::editing_busy(state)
            {
                return Some(LRESULT(0));
            }
            if engaged || state.drag.pending.borrow().is_some() {
                return Some(LRESULT(0));
            }
            state.canvas.borrow_mut().cancel_gesture();
            state.drag.origin.set(Some(point(lp)));
            SetCapture(hwnd);
        }
        WM_MOUSEMOVE if engaged => {
            if GetKeyState(VK_LBUTTON.0 as i32) >= 0 {
                cancel(hwnd, state);
            } else if let Some(origin) = state.drag.origin.get() {
                if state.drag.pending.borrow().is_none()
                    && crossed(
                        origin,
                        point(lp),
                        (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)),
                    )
                {
                    if let Err(error) = start(hwnd, state, profile()) {
                        cancel(hwnd, state);
                        eprintln!("native-drag: {error}");
                    }
                }
            }
        }
        _ => return None,
    }
    Some(LRESULT(0))
}
fn profile() -> Result<PathBuf, String> {
    let root = std::env::var_os("OTTER_NATIVE_PROFILE_DIR");
    let local = std::env::var_os("LOCALAPPDATA");
    crate::settings_profile::select(None, root.as_deref(), local.as_deref(), false)
        .map_err(str::to_owned)?
        .and_then(|selection| selection.path.parent().map(|path| path.to_owned()))
        .ok_or_else(|| "Missing native drag profile".into())
}
unsafe fn start(hwnd: HWND, state: &State, root: Result<PathBuf, String>) -> Result<(), String> {
    // 已取消但仍编码的任务也占有唯一槽位；禁止替换 owner 或积累后台任务。
    if state.drag.pending.borrow().is_some() {
        return Err("Native drag preparation is still active".into());
    }
    let root = root?;
    let name = super::export_naming::suggested_drag(state).ok_or("Cannot render drag filename")?;
    let stem = name
        .strip_suffix(".png")
        .ok_or("Invalid drag filename")?
        .to_owned();
    let canvas = state.canvas.borrow();
    let frame = state.resolved_display.borrow();
    let (image, editor) =
        super::display_snapshot::capture(&canvas, frame.as_ref()).ok_or("Missing drag image")?;
    let stamp = super::display_snapshot::Stamp::capture(&canvas, frame.as_ref());
    let input = drag_prepare::Input {
        width: image.width,
        height: image.height,
        pixels: image.pixels,
        editor,
        opacity: canvas.normal_opacity,
        root,
        stem,
        policy: state.cache_policy.get(),
    };
    drop(frame);
    drop(canvas);
    let job = drag_prepare::Job::start(input)?;
    *state.drag.pending.borrow_mut() = Some(Pending { job, stamp });
    if SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        state.drag.pending.borrow_mut().take();
        return Err("Cannot schedule native drag preparation".into());
    }
    Ok(())
}
unsafe fn cancel(hwnd: HWND, state: &State) {
    let owned_capture = state.drag.origin.take().is_some();
    let finished = {
        let pending = state.drag.pending.borrow();
        pending.as_ref().is_none_or(|pending| {
            pending.job.cancel();
            pending.job.is_finished()
        })
    };
    if finished {
        let pending = state.drag.pending.borrow_mut().take();
        let _ = KillTimer(Some(hwnd), TIMER);
        drop(pending);
    }
    // 普通取消不 join 正在 WIC 编码的线程；现有活动定时器继续负责完成后的回收。
    // 若后续编辑已取得捕获，旧任务回收不能释放新手势的鼠标捕获。
    if owned_capture && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
unsafe fn dispose(hwnd: HWND, state: &State) {
    cancel(hwnd, state);
    let pending = state.drag.pending.borrow_mut().take();
    let _ = KillTimer(Some(hwnd), TIMER);
    drop(pending); // HWND 销毁边界仍 join，不让任务失去 owner。
}

#[cfg(test)]
#[path = "drag_input_tests.rs"]
mod tests;
