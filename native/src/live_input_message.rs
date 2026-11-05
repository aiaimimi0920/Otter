//! 仅允许键鼠消息的 Win32 编码与投递；不广播、不抢焦点、不提升权限。
use super::{
    input_coordinates, input_security,
    input_target::{self, Destination},
    Target,
};
use crate::live_input::Button;
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
    Security(input_security::Denied),
    Destination(input_target::Error),
    Dpi,
    DeliveryFailed,
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
    pub fn key(code: u16, down: bool) -> Result<Self> {
        if !(1..=254).contains(&code) {
            return Err(Error::InvalidPayload);
        }
        let scan = unsafe { MapVirtualKeyW(u32::from(code), MAPVK_VK_TO_VSC) };
        Ok(Self {
            id: if down { WM_KEYDOWN } else { WM_KEYUP },
            wp: WPARAM(usize::from(code)),
            lp: LPARAM((1 | ((scan & 0xff) << 16) | if down { 0 } else { 0xc0000000 }) as isize),
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
    if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    deliver_destination(target, message, if message.moving { 50 } else { 250 })
}
pub fn release_to(target: &Destination, message: Message) -> Result<()> {
    if !message.releasing {
        return Err(Error::InvalidPayload);
    }
    deliver_destination(target, message, 50)
}
fn deliver_destination(target: &Destination, message: Message, timeout: u32) -> Result<()> {
    if target.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    target.preflight().map_err(Error::Destination)?;
    deliver_window(target.hwnd(), message, timeout)
}
fn deliver_window(window: HWND, message: Message, timeout: u32) -> Result<()> {
    // 鼠标坐标已按接收窗口映射；同上下文发送以阻止 Win32 再次 DPI 虚拟化。
    // 仅键盘没有空间坐标；RAII 在成功、超时和失败返回时均恢复 worker 上下文。
    let _dpi = if matches!(message.id, WM_KEYDOWN | WM_KEYUP) {
        None
    } else {
        Some(input_coordinates::Dpi::enter(window).map_err(|_| Error::Dpi)?)
    };
    let mut result = 0usize;
    let completed = unsafe {
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
