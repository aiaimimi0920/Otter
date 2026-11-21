//! 仅允许键鼠消息的 Win32 编码与投递；不广播、不抢焦点、不提升权限。
use super::{
    input_coordinates, input_security,
    input_target::{self, Destination},
    Target,
};
use crate::live_input::Button;
#[path = "live_input_modal.rs"]
mod modal;
pub(crate) use modal::{button_command, button_modal, motion_native, Modal};
use windows::Win32::UI::Controls::{WM_MOUSEHOVER, WM_MOUSELEAVE};
use windows::Win32::{
    Foundation::*,
    System::Threading::GetCurrentThreadId,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPayload,
    CoordinateRange,
    SameThread,
    SourceTransition,
    ControlRevoked,
    Security(input_security::Denied),
    Destination(input_target::Error),
    Dpi,
    DeliveryFailed,
    ModalPending,
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct Message {
    id: u32,
    wp: WPARAM,
    lp: LPARAM,
    moving: bool,
    releasing: bool,
}
impl Message {
    pub fn character(unit: u16) -> Result<Self> {
        if unit == 0 {
            return Err(Error::InvalidPayload);
        }
        Ok(Self {
            id: WM_CHAR,
            wp: WPARAM(usize::from(unit)),
            lp: LPARAM(1),
            moving: false,
            releasing: false,
        })
    }
    pub fn hover(point: [i32; 2], buttons: u8) -> Result<Self> {
        Ok(Self {
            id: WM_MOUSEHOVER,
            ..Self::motion(point, buttons)?
        })
    }
    pub fn leave() -> Self {
        Self {
            id: WM_MOUSELEAVE,
            wp: WPARAM(0),
            lp: LPARAM(0),
            moving: true,
            releasing: true,
        }
    }
    /// 本地浏览器退出悬浮时投递到 root；虚拟坐标在客户区外，不移动系统指针。
    pub(crate) fn exit_motion() -> Self {
        Self {
            id: WM_MOUSEMOVE,
            wp: WPARAM(0),
            lp: LPARAM(u32::MAX as isize), // 两个有符号 16 位坐标均为 -1。
            moving: true,
            releasing: true,
        }
    }
    pub fn key(code: u16, down: bool) -> Result<Self> {
        if !(1..=254).contains(&code) {
            return Err(Error::InvalidPayload);
        }
        let scan = unsafe { MapVirtualKeyW(u32::from(code), MAPVK_VK_TO_VSC_EX) };
        // 部分布局把导航 VK 映射到数字键盘的基础扫描码，即使请求 VSC_EX 也不带 E0。
        // 合同已区分导航 VK 与 VK_NUMPAD*，规范导航键仍须第24位；E1/Pause 不混同。
        // 不据此猜测数字键盘 Enter 或通用 VK_CONTROL/VK_MENU 的物理左右身份。
        let extended =
            u32::from(scan & 0xff00 == 0xe000 || matches!(code, 0x21..=0x28 | 0x2d | 0x2e)) << 24;
        Ok(Self {
            id: if down { WM_KEYDOWN } else { WM_KEYUP },
            wp: WPARAM(usize::from(code)),
            lp: LPARAM(
                (1 | ((scan & 0xff) << 16) | extended | if down { 0 } else { 0xc0000000 }) as isize,
            ),
            moving: false,
            releasing: !down,
        })
    }
    /// point 必须已转换到接收窗口客户区；不接受捕获帧像素直接充当客户区坐标。
    pub fn motion(point: [i32; 2], buttons: u8) -> Result<Self> {
        Ok(Self {
            id: WM_MOUSEMOVE,
            wp: WPARAM(key_state(buttons)?),
            lp: point_param(point)?,
            moving: true,
            releasing: false,
        })
    }
    pub fn button(
        point: [i32; 2],
        buttons: u8,
        button: Button,
        down: bool,
        double: bool,
    ) -> Result<Self> {
        if !down && double {
            return Err(Error::InvalidPayload);
        }
        let id = match (button, down, double) {
            (Button::Left, true, false) => WM_LBUTTONDOWN,
            (Button::Left, true, true) => WM_LBUTTONDBLCLK,
            (Button::Left, false, _) => WM_LBUTTONUP,
            (Button::Right, true, false) => WM_RBUTTONDOWN,
            (Button::Right, true, true) => WM_RBUTTONDBLCLK,
            (Button::Right, false, _) => WM_RBUTTONUP,
            (Button::Middle, true, false) => WM_MBUTTONDOWN,
            (Button::Middle, true, true) => WM_MBUTTONDBLCLK,
            (Button::Middle, false, _) => WM_MBUTTONUP,
        };
        Ok(Self {
            id,
            wp: WPARAM(key_state(buttons)?),
            lp: point_param(point)?,
            moving: false,
            releasing: !down,
        })
    }
    /// 滚轮 lParam 是屏幕坐标，而非客户区坐标。超出 signed 16-bit 时拒绝截断。
    pub fn wheel(screen: [i32; 2], buttons: u8, delta: i32, horizontal: bool) -> Result<Self> {
        if delta == 0 || delta.unsigned_abs() > 1200 {
            return Err(Error::InvalidPayload);
        }
        Ok(Self {
            id: if horizontal {
                WM_MOUSEHWHEEL
            } else {
                WM_MOUSEWHEEL
            },
            wp: WPARAM(key_state(buttons)? | ((delta as i16 as u16 as usize) << 16)),
            lp: point_param(screen)?,
            moving: false,
            releasing: false,
        })
    }
}
fn key_state(buttons: u8) -> Result<usize> {
    if buttons & !7 != 0 {
        return Err(Error::InvalidPayload);
    }
    Ok(usize::from(buttons & 3) | (usize::from(buttons & 4) << 2))
}
fn point_param(point: [i32; 2]) -> Result<LPARAM> {
    let [x, y] = point;
    let x = i16::try_from(x).map_err(|_| Error::CoordinateRange)?;
    let y = i16::try_from(y).map_err(|_| Error::CoordinateRange)?;
    Ok(LPARAM(
        (u32::from(x as u16) | (u32::from(y as u16) << 16)) as isize,
    ))
}
/// 必须由显式授权后的独立 worker 调用；不得 AttachThreadInput 到目标消息队列。
/// 此入口投递到绑定 root；已验证的子控件使用 send_to。
pub fn send(target: &Target, message: Message) -> Result<()> {
    if unsafe { IsIconic(target.hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    deliver(target, message, if message.moving { 50 } else { 250 })
}
/// 只准释放边沿，不运行按钮点击补偿；输入控制 owner 负责记录未知/失败的清理结果。
pub fn release(target: &Target, message: Message) -> Result<()> {
    if !message.releasing {
        return Err(Error::InvalidPayload);
    }
    deliver(target, message, 50)
}
fn deliver(target: &Target, message: Message, timeout: u32) -> Result<()> {
    // 同线程调用绕过超时，必须在调用系统 API 前拒绝。
    if target.identity.thread == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    input_security::preflight(target).map_err(Error::Security)?;
    deliver_window(target.hwnd(), message, timeout)
}
/// 向已绑定后代发送；仍需显式授权，不能把 Destination::bind 当成启用控制。
pub fn send_to(target: &Destination, message: Message) -> Result<()> {
    send_to_with_timeout(target, message, if message.moving { 50 } else { 250 })
}
pub(crate) fn send_to_with_timeout(
    target: &Destination,
    message: Message,
    timeout: u32,
) -> Result<()> {
    if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    deliver_destination(target, message, timeout)
}
pub fn release_to(target: &Destination, message: Message) -> Result<()> {
    release_to_with_timeout(target, message, 50)
}
pub(crate) fn release_to_with_timeout(
    target: &Destination,
    message: Message,
    timeout: u32,
) -> Result<()> {
    if !message.releasing {
        return Err(Error::InvalidPayload);
    }
    let result = deliver_destination(target, message, timeout);
    // 菜单可在身份预检之后、DPI 查询或同步 Leave 期间销毁。
    // 仅悬浮清理以 HWND 已不存在作为终态；不重试、不重新绑定复用句柄，
    // 按键/按钮释放及仍存活窗口的超时继续保留原失败。
    if result.is_err()
        && message.id == WM_MOUSELEAVE
        && !unsafe { IsWindow(Some(target.hwnd())) }.as_bool()
    {
        return Ok(());
    }
    result
}
fn deliver_destination(target: &Destination, message: Message, timeout: u32) -> Result<()> {
    if target.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    target.preflight().map_err(Error::Destination)?;
    // 隐藏菜单已结束这轮可见悬浮；其 UI 线程可能正在初始化下一个模态框。
    // 不再同步调用旧菜单的 Leave，避免把正常换窗变成超时。held 释放不走此终态。
    if message.id == WM_MOUSELEAVE && !unsafe { IsWindowVisible(target.hwnd()) }.as_bool() {
        return Ok(());
    }
    deliver_window(target.hwnd(), message, timeout)
}
fn deliver_window(window: HWND, message: Message, timeout: u32) -> Result<()> {
    // 鼠标坐标已按接收窗口映射；同上下文发送以阻止 Win32 再次 DPI 虚拟化。
    // 仅键盘没有空间坐标；RAII 在成功、超时和失败返回时均恢复 worker 上下文。
    let _dpi = if matches!(message.id, WM_KEYDOWN | WM_KEYUP | WM_CHAR) {
        None
    } else {
        Some(input_coordinates::Dpi::enter(window).map_err(|_| Error::Dpi)?)
    };
    let mut result = 0usize;
    let completed = unsafe {
        // 此 API 失败时不保证设置 LastError；先清零，避免把旧错误当成当前原因。
        SetLastError(ERROR_SUCCESS);
        SendMessageTimeoutW(
            window,
            message.id,
            message.wp,
            message.lp,
            SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
            timeout,
            Some(&mut result),
        )
    };
    // 消息处理返回 0 也可能成功。失败包含超时/UIPI 等，不能把它误报为已拒绝送达。
    if completed.0 == 0 {
        let error = unsafe { GetLastError() }.0;
        eprintln!(
            "native-live-message-failed: message=0x{:04X} win32={} timeout_ms={timeout}",
            message.id, error
        );
        Err(Error::DeliveryFailed)
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "live_input_delivery_dpi_tests.rs"]
mod delivery_dpi_tests;
#[cfg(test)]
#[path = "live_input_message_tests.rs"]
mod tests;
