//! 选区目标绑定的单步滚轮传输；不移动指针、不抢焦点、不广播，也不假定消息成功等于已滚动。
use super::Region;
use crate::long_capture::Axis;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::{
    Foundation::*,
    System::Threading::*,
    UI::{Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    window: isize,
    root: isize,
    process: u32,
    thread: u32,
    bounds: (i32, i32, i32, i32),
    child_bounds: (i32, i32, i32, i32),
    dpi: u32,
}
/// 仅保存标识与物理坐标，不持有 HWND 所有权；每次操作重新检查，不跨线程传递 UI 状态。
#[derive(Clone)]
pub struct Target {
    identity: Identity,
    point: (i32, i32),
}
fn center(region: Region) -> Result<(i32, i32), String> {
    if region.width <= 0 || region.height <= 0 {
        return Err("Invalid scroll region".into());
    }
    region
        .left
        .checked_add(region.width)
        .ok_or("Scroll region overflow")?;
    region
        .top
        .checked_add(region.height)
        .ok_or("Scroll region overflow")?;
    let x = region
        .left
        .checked_add(region.width / 2)
        .ok_or("Scroll point overflow")?;
    let y = region
        .top
        .checked_add(region.height / 2)
        .ok_or("Scroll point overflow")?;
    // WM_MOUSE(W)HEEL 的坐标是有符号 16 位，拒绝截断到另一显示器。
    i16::try_from(x).map_err(|_| "Scroll point exceeds message coordinate range")?;
    i16::try_from(y).map_err(|_| "Scroll point exceeds message coordinate range")?;
    Ok((x, y))
}
unsafe fn identity(window: HWND) -> Result<Identity, String> {
    if !IsWindow(Some(window)).as_bool() {
        return Err("Scroll target closed".into());
    }
    let root = GetAncestor(window, GA_ROOT);
    if !IsWindowVisible(root).as_bool()
        || !IsWindowVisible(window).as_bool()
        || !IsWindowEnabled(window).as_bool()
    {
        return Err("Scroll target unavailable".into());
    }
    let mut process = 0;
    let thread = GetWindowThreadProcessId(window, Some(&mut process));
    if process == 0 || thread == 0 {
        return Err("Scroll target identity unavailable".into());
    }
    let mut rect = RECT::default();
    GetWindowRect(root, &mut rect).map_err(|error| error.to_string())?;
    let mut child = RECT::default();
    GetWindowRect(window, &mut child).map_err(|error| error.to_string())?;
    Ok(Identity {
        window: window.0 as isize,
        root: root.0 as isize,
        process,
        thread,
        bounds: (rect.left, rect.top, rect.right, rect.bottom),
        child_bounds: (child.left, child.top, child.right, child.bottom),
        dpi: windows::Win32::UI::HiDpi::GetDpiForWindow(window),
    })
}
fn message(axis: Axis, forward: bool, point: (i32, i32)) -> (u32, WPARAM, LPARAM) {
    let (msg, delta) = match axis {
        Axis::Vertical => (WM_MOUSEWHEEL, -120i16),
        Axis::Horizontal => (WM_MOUSEHWHEEL, 120i16),
    };
    let delta = if forward { delta } else { -delta };
    let wp = WPARAM((u32::from(delta as u16) << 16) as usize);
    let lp = LPARAM(
        (u32::from(point.0 as i16 as u16) | (u32::from(point.1 as i16 as u16) << 16)) as isize,
    );
    (msg, wp, lp)
}
fn send_checked(
    target: &Target,
    current: Identity,
    caller_thread: u32,
    cancel: &AtomicBool,
    send: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Scroll cancelled".into());
    }
    if target.identity != current {
        return Err("Scroll target changed or moved".into());
    }
    // 同线程 SendMessageTimeout 会忽略超时；禁止该路径，而非声称仍有超时保护。
    if current.thread == caller_thread {
        return Err("Scroll target must use a different thread".into());
    }
    send()?;
    if cancel.load(Ordering::Acquire) {
        return Err("Scroll cancelled after delivery".into());
    }
    Ok(())
}
impl Target {
    pub(in crate::desktop) fn root(&self) -> isize {
        self.identity.root
    }
    /// 通知只提供采样提示；同一根窗口里的无关子窗也不能重新授权目标。
    pub(in crate::desktop) fn accepts_point(&self, point: POINT) -> Result<bool, String> {
        self.validate()?;
        unsafe {
            let hit = WindowFromPoint(point);
            let bound = HWND(self.identity.window as *mut _);
            Ok(GetAncestor(hit, GA_ROOT).0 as isize == self.identity.root
                && (hit == bound || IsChild(bound, hit).as_bool()))
        }
    }
    /// 仅更新同一目标内的物理滚轮坐标，不重新命中或授权另一个窗口。
    pub fn relocate(&self, region: Region) -> Result<Option<Self>, String> {
        self.validate()?;
        self.region_target(region)
    }
    fn region_target(&self, region: Region) -> Result<Option<Self>, String> {
        if region.width <= 0 || region.height <= 0 {
            return Err("Invalid moved viewfinder".into());
        }
        let moved_right = region
            .left
            .checked_add(region.width)
            .ok_or("Viewfinder coordinate overflow")?;
        let moved_bottom = region
            .top
            .checked_add(region.height)
            .ok_or("Viewfinder coordinate overflow")?;
        let (left, top, right, bottom) = self.identity.bounds;
        if region.left < left || region.top < top || moved_right > right || moved_bottom > bottom {
            return Ok(None);
        }
        // 超出原窗口或滚轮坐标范围只撤销滚动能力，不限制桌面取景框移动或授权新窗口。
        let Ok(point) = center(region) else {
            return Ok(None);
        };
        Ok(Some(Self {
            identity: self.identity,
            point,
        }))
    }
    pub fn validate(&self) -> Result<(), String> {
        let current = unsafe { identity(HWND(self.identity.window as *mut _))? };
        if current != self.identity {
            Err("Scroll target changed or moved".into())
        } else {
            Ok(())
        }
    }
    /// 必须在显示长截图控制窗口前、用户确认选区后绑定。禁止将 Otter 自己作为滚动目标。
    pub fn bind(region: Region) -> Result<Self, String> {
        let point = center(region)?;
        unsafe {
            let window = WindowFromPoint(POINT {
                x: point.0,
                y: point.1,
            });
            let identity = identity(window)?;
            if identity.process == GetCurrentProcessId() {
                return Err("Cannot auto-scroll an Otter window".into());
            }
            let (left, top, right, bottom) = identity.bounds;
            if region.left < left
                || region.top < top
                || region.left + region.width > right
                || region.top + region.height > bottom
            {
                return Err("Scroll region must stay inside one target window".into());
            }
            Ok(Self { identity, point })
        }
    }
    /// 只能从独立 worker 调用。一次发送一格，不提升权限、不重试或转发到父窗口。
    /// 返回成功仅证明消息交付，后续仍必须用截图分析判断实际运动。
    pub fn step(&self, axis: Axis, forward: bool, cancel: &AtomicBool) -> Result<(), String> {
        if cancel.load(Ordering::Acquire) {
            return Err("Scroll cancelled".into());
        }
        unsafe {
            let window = HWND(self.identity.window as *mut _);
            let current = identity(window)?;
            let (msg, wp, lp) = message(axis, forward, self.point);
            send_checked(self, current, GetCurrentThreadId(), cancel, || {
                SetLastError(ERROR_SUCCESS);
                let result = SendMessageTimeoutW(
                    window,
                    msg,
                    wp,
                    lp,
                    SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
                    150,
                    None,
                );
                if result.0 == 0 {
                    let code = GetLastError().0;
                    Err(format!("Scroll delivery failed or timed out ({code})"))
                } else {
                    Ok(())
                }
            })
        }
    }
}

#[cfg(test)]
#[path = "long_capture_scroll_tests.rs"]
mod tests;
