//! 源窗口内的只读命中/焦点路由；不抢焦点、不修改透明样式、不枚举桌面窗口。
use super::{
    input_coordinates::Dpi,
    input_target::{self, Destination},
    Target,
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
const MAX_DEPTH: usize = 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Target(input_target::Error),
    Transform,
    DepthLimit,
}
type Result<T> = std::result::Result<T, Error>;

/// root_client 必须由源 DPI 坐标转换获得。只下钻指定 root，不使用 WindowFromPoint。
pub fn pointer(root: &Target, root_client: [i32; 2]) -> Result<Destination> {
    root.validate()
        .map_err(|_| Error::Target(input_target::Error::SourceChanged))?;
    let _dpi = Dpi::enter(root.hwnd()).map_err(|_| Error::Transform)?;
    let mut screen = POINT {
        x: root_client[0],
        y: root_client[1],
    };
    if !unsafe { ClientToScreen(root.hwnd(), &mut screen) }.as_bool() {
        return Err(Error::Transform);
    }
    let mut current = root.hwnd();
    let mut remaining = 256;
    for _ in 0..MAX_DEPTH {
        let Some(child) = child_at(current, screen, &mut remaining)? else {
            return nearest(root, current);
        };
        if !unsafe { IsChild(root.hwnd(), child) }.as_bool() {
            return Err(Error::Target(input_target::Error::NotDescendant));
        }
        current = child;
    }
    Err(Error::DepthLimit)
}
/// 按直接子窗口 Z 序遍历。透明 root 不依赖系统命中 API 的空结果，也不改样式。
/// 全次查找最多检查 256 个子窗口，拒绝异常树而非无限遍历。
fn child_at(parent: HWND, screen: POINT, remaining: &mut usize) -> Result<Option<HWND>> {
    if !unsafe { IsWindowVisible(parent) }.as_bool() {
        return Ok(None);
    }
    let mut local = screen;
    let mut client = RECT::default();
    if !unsafe { ScreenToClient(parent, &mut local) }.as_bool() {
        return Err(Error::Transform);
    }
    unsafe { GetClientRect(parent, &mut client) }.map_err(|_| Error::Transform)?;
    if !contains(client, local) {
        return Ok(None);
    }
    let mut next = unsafe { GetWindow(parent, GW_CHILD) }.ok();
    while let Some(child) = next {
        if *remaining == 0 {
            return Err(Error::DepthLimit);
        }
        *remaining -= 1;
        if unsafe { GetParent(child) }.ok() != Some(parent) {
            return Err(Error::Transform);
        }
        let visible = unsafe { IsWindowVisible(child) }.as_bool();
        let enabled = unsafe { IsWindowEnabled(child) }.as_bool();
        let transparent =
            unsafe { GetWindowLongPtrW(child, GWL_EXSTYLE) } & WS_EX_TRANSPARENT.0 as isize != 0;
        if visible && enabled && !transparent {
            let mut bounds = RECT::default();
            unsafe { GetWindowRect(child, &mut bounds) }.map_err(|_| Error::Transform)?;
            if contains(bounds, screen) {
                return Ok(Some(child));
            }
        }
        next = unsafe { GetWindow(child, GW_HWNDNEXT) }.ok();
    }
    Ok(None)
}
fn contains(rect: RECT, point: POINT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}
/// 不调用 SetFocus/AttachThreadInput；别的窗口拥有焦点时只回退到原 root。
pub fn keyboard(root: &Target) -> Result<Destination> {
    root.validate()
        .map_err(|_| Error::Target(input_target::Error::SourceChanged))?;
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let candidate = if unsafe { GetGUIThreadInfo(root.identity.thread, &mut info) }.is_ok()
        && (info.hwndFocus == root.hwnd()
            || unsafe { IsChild(root.hwnd(), info.hwndFocus) }.as_bool())
    {
        info.hwndFocus
    } else {
        root.hwnd()
    };
    nearest(root, candidate)
}
/// 已有拖动/按键优先保留目标；失效只回退已授权 root，不能命中另一个控件。
pub fn retained(root: &Target, previous: &Destination) -> Result<Destination> {
    if !root.same_source(previous.root()) {
        return Err(Error::Target(input_target::Error::NotDescendant));
    }
    if previous.preflight().is_ok() {
        return Ok(previous.clone());
    }
    Destination::bind(root, root.hwnd()).map_err(Error::Target)
}
fn nearest(root: &Target, mut candidate: HWND) -> Result<Destination> {
    for _ in 0..MAX_DEPTH {
        match Destination::bind(root, candidate) {
            Ok(target) => return Ok(target),
            Err(error) if candidate == root.hwnd() => return Err(Error::Target(error)),
            Err(_) => {}
        }
        let parent = unsafe { GetParent(candidate) }.ok();
        candidate = match parent {
            Some(parent)
                if parent == root.hwnd() || unsafe { IsChild(root.hwnd(), parent) }.as_bool() =>
            {
                parent
            }
            _ => root.hwnd(),
        };
    }
    Err(Error::DepthLimit)
}
#[cfg(test)]
#[path = "live_input_routing_tests.rs"]
mod tests;
