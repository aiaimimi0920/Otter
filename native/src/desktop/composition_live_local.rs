//! Live 本地滚轮优先于源输入；共用钉图几何/合成接口，不改变采集像素或授权。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "composition_live_drag.rs"]
pub(super) mod drag;

pub(super) fn pressed(key: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(key.0 as i32) < 0 }
}
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<Result<()>> {
    if message != WM_MOUSEWHEEL || pressed(VK_SHIFT) || pressed(VK_LWIN) || pressed(VK_RWIN) {
        return None;
    }
    let (ctrl, alt) = (pressed(VK_CONTROL), pressed(VK_MENU));
    if ctrl == alt {
        return None;
    }
    // Ctrl/Alt 可能已经送达源；先走现有停止通道释放源按键，不能只吞掉滚轮。
    input::cancel(hwnd, state);
    if state.group_locked.get() || state.group_hidden.get() {
        return Some(Ok(()));
    }
    let delta = (wp.0 >> 16) as u16 as i16;
    if delta == 0 {
        return Some(Ok(()));
    }
    Some(if alt {
        let next = (state.opacity.get() + f32::from(delta) * 0.001).clamp(0.0, 1.0);
        // 在可能重入的合成调用前提交期望值，嵌套 wheel 才能在最新值上累加。
        // 合成失败沿既有设备失败退出；补偿 paint 总是读取最新期望值。
        state.opacity.set(next);
        state.with_host(|host| host.set_opacity(next)).map(|_| {
            if !state.closed.get() {
                // Host 暂借出时保存期望值；补偿 paint 会重新应用，不丢嵌套消息。
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        })
    } else {
        // 几何调整失败不是采集设备失败，不能因此销毁用户 Live。
        let _ = zoom(hwnd, delta, lp);
        Ok(())
    })
}
unsafe fn zoom(hwnd: HWND, delta: i16, lp: LPARAM) -> Result<()> {
    let before = client_rect(hwnd)?;
    let pointer = (lp.0 as u16 as i16 as i32, (lp.0 >> 16) as u16 as i16 as i32);
    let Some(next) = crate::pin_geometry::zoom(before, pointer, f32::from(delta) / 120.0) else {
        return Ok(());
    };
    if before == next {
        return Ok(());
    }
    place(hwnd, next)
}
pub(super) unsafe fn client_rect(hwnd: HWND) -> Result<crate::pin_geometry::PinRect> {
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?;
    let mut origin = POINT::default();
    if !ClientToScreen(hwnd, &mut origin).as_bool() {
        return Err(Error::from_win32());
    }
    Ok(crate::pin_geometry::PinRect {
        x: origin.x,
        y: origin.y,
        width: client.right.max(0) as u32,
        height: client.bottom.max(0) as u32,
    })
}
unsafe fn place(hwnd: HWND, next: crate::pin_geometry::PinRect) -> Result<()> {
    place_while(hwnd, next, || true)
}
unsafe fn place_while(
    hwnd: HWND,
    next: crate::pin_geometry::PinRect,
    current: impl Fn() -> bool,
) -> Result<()> {
    let right = next
        .x
        .checked_add(next.width as i32)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Live zoom x overflow"))?;
    let bottom = next
        .y
        .checked_add(next.height as i32)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Live zoom y overflow"))?;
    let mut original = RECT::default();
    GetWindowRect(hwnd, &mut original)?;
    let result = placement::place_client_while(
        hwnd,
        RECT {
            left: next.x,
            top: next.y,
            right,
            bottom,
        },
        &current,
    );
    if result.is_err() && current() {
        // 定位可能已部分完成；恢复原外框。系统拒绝恢复时保留实际 HWND 几何，
        // 后续保存/输入始终重新读取实际客户区，不提交虚构的成功尺寸。
        let _ = SetWindowPos(
            hwnd,
            None,
            original.left,
            original.top,
            original.right - original.left,
            original.bottom - original.top,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOSENDCHANGING,
        );
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    result
}

#[cfg(test)]
#[path = "composition_live_local_tests.rs"]
pub(in crate::desktop) mod tests;
