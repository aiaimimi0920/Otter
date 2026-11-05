//! 只向实际父窗口发送值变化边沿；调用 Win32/父通知前不持有 State 引用，防止重入销毁。
use super::*;
use windows::Win32::UI::Shell::*;
const ID: usize = 0x4f545352;
/// 只区分系统兼容鼠标消息的来源，不作为输入授权依据；笔和真实鼠标保持原行为。
pub(super) fn is_touch_promotion(extra: usize) -> bool {
    extra & 0xffffff00 == 0xff515700 && extra & 0x80 != 0
}
pub(super) fn install(hwnd: HWND) -> Result<()> {
    // 让系统完整拥有 pointer 序列：垂直识别为父滚动手势，水平仍提升为 Slider 输入。
    unsafe {
        use windows::Win32::{System::SystemServices::*, UI::Input::Touch::*};
        SetGestureConfig(
            hwnd,
            0,
            &[GESTURECONFIG {
                dwID: GID_PAN,
                dwWant: (GC_PAN | GC_PAN_WITH_SINGLE_FINGER_VERTICALLY | GC_PAN_WITH_INERTIA).0,
                dwBlock: (GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY | GC_PAN_WITH_GUTTER).0,
            }],
            std::mem::size_of::<GESTURECONFIG>() as u32,
        )?;
    }
    if unsafe { SetWindowSubclass(hwnd, Some(procedure), ID, 0) }.as_bool() {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}
pub(super) unsafe fn cancel(hwnd: HWND) {
    let mut click = false;
    if !state(hwnd).is_null() {
        (*state(hwnd)).drag = None;
        (*state(hwnd)).drag_touch = false;
        click = std::mem::take(&mut (*state(hwnd)).click_active);
    }
    if click {
        click_message(hwnd, CLICK_CANCEL, LPARAM(0));
    }
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
unsafe fn click_message(hwnd: HWND, message: u32, point: LPARAM) {
    if let Ok(parent) = GetParent(hwnd) {
        SendMessageW(
            parent,
            message,
            Some(WPARAM(point.0 as usize)),
            Some(LPARAM(hwnd.0 as isize)),
        );
    }
}
unsafe fn notify(hwnd: HWND, change: bool) {
    if !alive(hwnd) {
        return;
    }
    if let Ok(parent) = GetParent(hwnd) {
        SendMessageW(
            parent,
            NOTIFY,
            Some(WPARAM(usize::from(change))),
            Some(LPARAM(hwnd.0 as isize)),
        );
    }
}
unsafe fn finish_drag(hwnd: HWND) {
    let marker = GetPropW(hwnd, OWNER);
    let value = (*state(hwnd)).value;
    let changed = (*state(hwnd))
        .drag
        .take()
        .is_some_and(|start| start != value);
    cancel(hwnd);
    // ReleaseCapture 可以重入；只有仍属于本手势的值才允许提交。
    if changed
        && alive(hwnd)
        && GetPropW(hwnd, OWNER) == marker
        && IsWindowEnabled(hwnd).as_bool()
        && (*state(hwnd)).value == value
    {
        notify(hwnd, true);
    }
}
unsafe fn checkpoint_drag(hwnd: HWND) {
    if GetCapture() != hwnd {
        return;
    }
    let value = (*state(hwnd)).value;
    if (*state(hwnd)).drag.is_some_and(|before| before != value) {
        // 切焦提交当前边沿，但鼠标手势仍在；后续移动/抬起仍归原 Slider。
        (*state(hwnd)).drag = Some(value);
        notify(hwnd, true);
    }
}
unsafe fn update(hwnd: HWND, value: f64, commit: bool) {
    if !alive(hwnd) {
        return;
    }
    if (*state(hwnd)).value == value {
        return;
    }
    (*state(hwnd)).value = value;
    let marker = GetPropW(hwnd, OWNER);
    let _ = InvalidateRect(Some(hwnd), None, false);
    notify(hwnd, false);
    // input 接收者可以同步禁用、关闭或替换控件；不能再向旧 owner 发送 change。
    if commit
        && alive(hwnd)
        && GetPropW(hwnd, OWNER) == marker
        && IsWindowEnabled(hwnd).as_bool()
        && (*state(hwnd)).value == value
    {
        // 键盘已经提交本值；之后继续拖动仍可提交，但单纯抬起不再重复 change。
        if (*state(hwnd)).drag.is_some() {
            (*state(hwnd)).drag = Some(value);
        }
        notify(hwnd, true);
    }
}
unsafe fn point(hwnd: HWND, lp: LPARAM) -> Option<f64> {
    let (left, right, _, _) = paint::track(hwnd)?;
    let x = (lp.0 as u32 & 0xffff) as i16 as f64;
    let fraction = if right <= left {
        0.
    } else {
        (x - left as f64) / (right - left) as f64
    };
    (*state(hwnd)).range.at_fraction(fraction).ok()
}
unsafe fn key(hwnd: HWND, key: u16) -> bool {
    let range = (*state(hwnd)).range;
    let value = (*state(hwnd)).value;
    let next = match VIRTUAL_KEY(key) {
        VK_LEFT | VK_DOWN => range.advance(value, false),
        VK_RIGHT | VK_UP => range.advance(value, true),
        VK_HOME => range.normalize(range.min()),
        VK_END => range.normalize(range.max()),
        VK_PRIOR | VK_NEXT => {
            let delta = ((range.max() - range.min()) / 10.).max(range.step());
            range.normalize(if key == VK_PRIOR.0 {
                (value + delta).min(range.max())
            } else {
                (value - delta).max(range.min())
            })
        }
        _ => return false,
    };
    if let Ok(next) = next {
        update(hwnd, next, true);
    }
    true
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if crate::desktop::session_end::blocks_input(message) {
        return LRESULT(0);
    }
    if message == WM_NCDESTROY {
        // 完成回调也可以销毁 HWND；即使已清 click_active，也必须撤销父候选。
        click_message(hwnd, CLICK_CANCEL, LPARAM(0));
        cancel(hwnd);
        let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
        take_state(hwnd);
        return DefSubclassProc(hwnd, message, wp, lp);
    }
    if !alive(hwnd) {
        return DefSubclassProc(hwnd, message, wp, lp);
    }
    if matches!(message, WM_KEYDOWN | WM_CHAR | WM_KEYUP) && wp.0 == VK_TAB.0 as usize {
        if message == WM_KEYDOWN
            && lp.0 as usize & (1 << 30) == 0
            && GetFocus() == hwnd
            && IsWindowEnabled(hwnd).as_bool()
        {
            let marker = GetPropW(hwnd, OWNER);
            checkpoint_drag(hwnd);
            if alive(hwnd) && GetPropW(hwnd, OWNER) == marker && IsWindowEnabled(hwnd).as_bool() {
                if let Ok(parent) = GetParent(hwnd) {
                    SendMessageW(
                        parent,
                        crate::desktop::surface_edit_messages::NAVIGATE,
                        Some(WPARAM(usize::from(GetKeyState(VK_SHIFT.0 as i32) < 0))),
                        Some(LPARAM(hwnd.0 as isize)),
                    );
                }
            }
        }
        return LRESULT(0);
    }
    if message == WM_KILLFOCUS {
        let marker = GetPropW(hwnd, OWNER);
        checkpoint_drag(hwnd);
        if !alive(hwnd) || GetPropW(hwnd, OWNER) != marker {
            return LRESULT(0);
        }
    }
    if matches!(message, WM_CANCELMODE | WM_CAPTURECHANGED)
        || (matches!(message, WM_ENABLE | WM_SHOWWINDOW) && wp.0 == 0)
    {
        cancel(hwnd);
        if !alive(hwnd) {
            return LRESULT(0);
        }
    }
    if let Some(result) = paint::message(hwnd, message, wp) {
        return result;
    }
    if IsWindowEnabled(hwnd).as_bool() {
        match message {
            WM_GETDLGCODE => return LRESULT((DLGC_WANTARROWS | DLGC_WANTCHARS) as isize),
            WM_KEYDOWN if GetFocus() == hwnd && key(hwnd, wp.0 as u16) => return LRESULT(0),
            WM_LBUTTONDOWN => {
                if GetCapture() == hwnd && (*state(hwnd)).drag.is_some() {
                    return LRESULT(0);
                }
                // 在 SetFocus/SetCapture 的重入之前保存消息来源，完成时不重新猜测。
                let touch = is_touch_promotion(GetMessageExtraInfo().0 as usize);
                let marker = GetPropW(hwnd, OWNER);
                let _ = SetFocus(Some(hwnd));
                if !alive(hwnd)
                    || GetPropW(hwnd, OWNER) != marker
                    || !IsWindowEnabled(hwnd).as_bool()
                {
                    return LRESULT(0);
                }
                SetCapture(hwnd);
                if !alive(hwnd)
                    || GetPropW(hwnd, OWNER) != marker
                    || GetCapture() != hwnd
                    || !IsWindowEnabled(hwnd).as_bool()
                {
                    if alive(hwnd) && GetPropW(hwnd, OWNER) == marker {
                        cancel(hwnd);
                    }
                    return LRESULT(0);
                }
                (*state(hwnd)).drag = Some((*state(hwnd)).value);
                (*state(hwnd)).drag_touch = touch;
                (*state(hwnd)).click_active = !touch;
                if !touch {
                    click_message(hwnd, CLICK_BEGIN, lp);
                }
                if !alive(hwnd) || GetPropW(hwnd, OWNER) != marker || GetCapture() != hwnd {
                    return LRESULT(0);
                }
                if let Some(value) = point(hwnd, lp) {
                    update(hwnd, value, false);
                }
                return LRESULT(0);
            }
            WM_MOUSEMOVE if GetCapture() == hwnd && (*state(hwnd)).drag.is_some() => {
                if is_touch_promotion(GetMessageExtraInfo().0 as usize) != (*state(hwnd)).drag_touch
                {
                    return LRESULT(0);
                }
                if let Some(value) = point(hwnd, lp) {
                    update(hwnd, value, false);
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP if GetCapture() == hwnd => {
                if is_touch_promotion(GetMessageExtraInfo().0 as usize) != (*state(hwnd)).drag_touch
                {
                    return LRESULT(0);
                }
                let marker = GetPropW(hwnd, OWNER);
                // 正常收尾时 WM_CAPTURECHANGED 不能撤销待派发的 click；真实取消仍会撤销。
                let click = std::mem::take(&mut (*state(hwnd)).click_active);
                finish_drag(hwnd);
                if click
                    && alive(hwnd)
                    && GetPropW(hwnd, OWNER) == marker
                    && IsWindowEnabled(hwnd).as_bool()
                {
                    click_message(hwnd, CLICK_END, lp);
                } else if click {
                    click_message(hwnd, CLICK_CANCEL, lp);
                }
                return LRESULT(0);
            }
            _ => {}
        }
    }
    let result = DefSubclassProc(hwnd, message, wp, lp);
    if matches!(message, WM_SETFOCUS | WM_KILLFOCUS | WM_ENABLE) && IsWindow(Some(hwnd)).as_bool() {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    result
}
