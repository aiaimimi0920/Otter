//! 可见门只触及本测试自启PID/精确类名；像素前后核遮挡，不采集其他应用内容、不抢焦点。
use std::{os::windows::io::AsRawHandle, process::Child};
use windows::{
    core::{w, BOOL},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
        System::JobObjects::*,
        UI::WindowsAndMessaging::*,
    },
};
pub struct Owned {
    pub child: Child,
    job: HANDLE,
}
impl Owned {
    pub fn new(child: Child) -> Self {
        unsafe {
            // 先建立child回收guard；创建/配置Job失败也不能遗留自己的全屏进程。
            let mut owner = Self {
                child,
                job: HANDLE::default(),
            };
            owner.job = CreateJobObjectW(None, None).unwrap();
            let job = owner.job;
            let info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
                BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                    LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                    ..Default::default()
                },
                ..Default::default()
            };
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
            )
            .unwrap();
            AssignProcessToJobObject(job, HANDLE(owner.child.as_raw_handle())).unwrap();
            owner
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        unsafe {
            let _ = CloseHandle(self.job);
        }
    }
}
pub fn matches(hwnd: HWND, pid: u32) -> bool {
    unsafe {
        let mut owner = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut owner));
        let mut class = [0u16; 64];
        let length = GetClassNameW(hwnd, &mut class);
        owner == pid
            && IsWindow(Some(hwnd)).as_bool()
            && length > 0
            && &class[..length as usize] == w!("Otter.Native.Tile").as_wide()
    }
}
struct Search {
    pid: u32,
    window: Option<HWND>,
}
unsafe extern "system" fn enumerate(hwnd: HWND, lp: LPARAM) -> BOOL {
    let s = &mut *(lp.0 as *mut Search);
    if matches(hwnd, s.pid) && IsWindowVisible(hwnd).as_bool() {
        s.window = Some(hwnd);
    }
    BOOL(1)
}
pub fn find(pid: u32) -> Option<HWND> {
    let mut s = Search { pid, window: None };
    unsafe {
        let _ = EnumWindows(Some(enumerate), LPARAM((&mut s as *mut Search) as isize));
    }
    s.window
}
/// 与既有Art前台门相同的真实可枚举启动器模式；没有UIA/线程附加/foreground-lock绕过。
pub struct Launcher(pub HWND);
impl Launcher {
    pub fn new() -> Self {
        unsafe {
            Self(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    &windows::core::HSTRING::from(format!(
                        "Otter Tile Test Input {}",
                        std::process::id()
                    )),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    420,
                    40,
                    440,
                    100,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            )
        }
    }
    pub fn transfer(&self, child: HWND, pid: u32) -> bool {
        unsafe {
            if GetForegroundWindow() != self.0 {
                return false;
            }
            assert!(matches(child, pid));
            // 仅在启动器已经由Computer Use真实激活后，转给已核归属的同测试子窗口一次。
            let allowed = SetForegroundWindow(child).as_bool();
            let foreground = GetForegroundWindow();
            assert!(allowed, "owned launcher child activation refused; launcher={:?} child={child:?} actual={foreground:?}", self.0);
            assert_eq!(foreground, child);
            true
        }
    }
}
impl Drop for Launcher {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}
pub fn pump() {
    unsafe {
        let mut m = MSG::default();
        while PeekMessageW(&mut m, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}
struct Screen(HDC);
impl Drop for Screen {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(None, self.0);
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum ProbeError {
    Identity,
    Hidden,
    Rectangle([i32; 4]),
    Occluded,
    Desktop,
}
pub fn samples(hwnd: HWND, pid: u32, rect: [i32; 4]) -> Result<[[u8; 3]; 4], ProbeError> {
    unsafe {
        if !matches(hwnd, pid) {
            return Err(ProbeError::Identity);
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return Err(ProbeError::Hidden);
        }
        let mut actual = RECT::default();
        GetWindowRect(hwnd, &mut actual).map_err(|_| ProbeError::Desktop)?;
        let observed = [
            actual.left,
            actual.top,
            actual.right - actual.left,
            actual.bottom - actual.top,
        ];
        if observed != rect {
            return Err(ProbeError::Rectangle(observed));
        }
        let points = [
            [rect[0] + rect[2] / 4, rect[1] + rect[3] / 4],
            [rect[0] + rect[2] * 3 / 4, rect[1] + rect[3] / 4],
            [rect[0] + rect[2] / 4, rect[1] + rect[3] * 3 / 4],
            [rect[0] + rect[2] * 3 / 4, rect[1] + rect[3] * 3 / 4],
        ];
        let owns = || {
            points
                .iter()
                .all(|p| WindowFromPoint(POINT { x: p[0], y: p[1] }) == hwnd)
        };
        if !owns() {
            return Err(ProbeError::Occluded);
        }
        DwmFlush().map_err(|_| ProbeError::Desktop)?;
        let dc = Screen(GetDC(None));
        if dc.0.is_invalid() {
            return Err(ProbeError::Desktop);
        }
        let mut result = [[0; 3]; 4];
        for (p, out) in points.iter().zip(&mut result) {
            if !matches(hwnd, pid) || !owns() {
                return Err(ProbeError::Occluded);
            }
            let pixel = GetPixel(dc.0, p[0], p[1]).0;
            if pixel == u32::MAX {
                return Err(ProbeError::Desktop);
            }
            *out = [pixel as u8, (pixel >> 8) as u8, (pixel >> 16) as u8];
        }
        if matches(hwnd, pid) && owns() {
            Ok(result)
        } else {
            Err(ProbeError::Occluded)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visible_probe_never_accepts_a_foreign_pid_or_an_owned_non_tile_window() {
        unsafe {
            let h = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!("probe negative"),
                WS_POPUP,
                -30000,
                -30000,
                2,
                2,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            assert!(!matches(h, std::process::id()));
            assert!(!matches(h, 0));
            assert!(samples(h, std::process::id(), [-30000, -30000, 2, 2]).is_err());
            DestroyWindow(h).unwrap();
        }
    }
}
