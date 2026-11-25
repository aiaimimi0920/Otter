//! Win32 encoding/delivery restricted to keyboard and mouse messages; no broadcasts, focus stealing, or privilege elevation.
use super::{
    input_coordinates, input_security,
    input_target::{self, Destination},
    Target,
};
use crate::live_input::Button;
#[path = "live_input_modal.rs"]
pub mod modal;
pub use modal::{button_command, button_modal, motion_native, Modal};
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
    /// Deliver to root when local-browser hover exits; virtual coordinates lie outside the client area without moving the system pointer.
    pub fn exit_motion() -> Self {
        Self {
            id: WM_MOUSEMOVE,
            wp: WPARAM(0),
            lp: LPARAM(u32::MAX as isize), // Both signed 16-bit coordinates are -1.
            moving: true,
            releasing: true,
        }
    }
    pub fn key(code: u16, down: bool) -> Result<Self> {
        if !(1..=254).contains(&code) {
            return Err(Error::InvalidPayload);
        }
        let scan = unsafe { MapVirtualKeyW(u32::from(code), MAPVK_VK_TO_VSC_EX) };
        // Some layouts map navigation VKs to basic numpad scan codes without E0, even when VSC_EX is requested.
        // The contract distinguishes navigation VKs from VK_NUMPAD*; canonical navigation keys still need bit 24, without conflating E1/Pause.
        // Do not infer physical left/right identity for numpad Enter or generic VK_CONTROL/VK_MENU from this.
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
    /// point must already be converted to the receiving window's client area; capture-frame pixels cannot stand in for client coordinates.
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
    /// Wheel lParam uses screen coordinates, not client coordinates. Reject truncation outside the signed 16-bit range.
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
/// Must be called by an independent worker after explicit authorization; never AttachThreadInput to the target message queue.
/// This entry point delivers to the bound root; validated child controls use send_to.
pub fn send(target: &Target, message: Message) -> Result<()> {
    if unsafe { IsIconic(target.hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    deliver(target, message, if message.moving { 50 } else { 250 })
}
/// Release edges only, without button-click compensation; the input-control owner records unknown/failed cleanup outcomes.
pub fn release(target: &Target, message: Message) -> Result<()> {
    if !message.releasing {
        return Err(Error::InvalidPayload);
    }
    deliver(target, message, 50)
}
fn deliver(target: &Target, message: Message, timeout: u32) -> Result<()> {
    // Same-thread calls bypass the timeout and must be rejected before invoking the system API.
    if target.identity.thread == unsafe { GetCurrentThreadId() } {
        return Err(Error::SameThread);
    }
    input_security::preflight(target).map_err(Error::Security)?;
    deliver_window(target.hwnd(), message, timeout)
}
/// Send to a bound descendant; explicit authorization is still required, and Destination::bind does not enable control.
pub fn send_to(target: &Destination, message: Message) -> Result<()> {
    send_to_with_timeout(target, message, if message.moving { 50 } else { 250 })
}
pub fn send_to_with_timeout(target: &Destination, message: Message, timeout: u32) -> Result<()> {
    if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
        return Err(Error::SourceTransition);
    }
    deliver_destination(target, message, timeout)
}
pub fn release_to(target: &Destination, message: Message) -> Result<()> {
    release_to_with_timeout(target, message, 50)
}
pub fn release_to_with_timeout(target: &Destination, message: Message, timeout: u32) -> Result<()> {
    if !message.releasing {
        return Err(Error::InvalidPayload);
    }
    let result = deliver_destination(target, message, timeout);
    // A menu may be destroyed after identity preflight, during DPI queries or synchronous Leave.
    // Only hover cleanup treats a nonexistent HWND as terminal; no retries or rebinding of reused handles,
    // while key/button-release failures and timeouts on still-live windows retain their original failures.
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
    // A hidden menu has ended this visible-hover instance; its UI thread may be initializing the next modal dialog.
    // Stop synchronously calling Leave on the old menu to avoid turning normal window changes into timeouts. Held releases do not use this terminal state.
    if message.id == WM_MOUSELEAVE && !unsafe { IsWindowVisible(target.hwnd()) }.as_bool() {
        return Ok(());
    }
    deliver_window(target.hwnd(), message, timeout)
}
fn deliver_window(window: HWND, message: Message, timeout: u32) -> Result<()> {
    // Mouse coordinates are already mapped for the receiver; send in the same context to prevent another Win32 DPI virtualization pass.
    // Only keyboard input lacks spatial coordinates; RAII restores the worker context on success, timeout, and failure returns.
    let _dpi = if matches!(message.id, WM_KEYDOWN | WM_KEYUP | WM_CHAR) {
        None
    } else {
        Some(input_coordinates::Dpi::enter(window).map_err(|_| Error::Dpi)?)
    };
    let mut result = 0usize;
    let completed = unsafe {
        // This API need not set LastError on failure; clear it first so stale errors are not mistaken for the current cause.
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
    // A message-handler return of 0 may still be successful. Failures include timeout/UIPI and must not be misreported as rejected delivery.
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
