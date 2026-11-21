//! Chromium 本地页面滚动：独立受限 UIA child，不重发已执行或结果未知的滚动。
use crate::{
    bounded_worker_process as bounded, live_recovery_pipe::Pipe, live_window_target::Target,
};
use std::{
    io::{self, Read, Write},
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
#[path = "live_browser_scroll_process.rs"]
pub mod process;
#[path = "live_browser_scroll_protocol.rs"]
mod protocol;
#[path = "live_browser_scroll_provider.rs"]
mod provider;
#[path = "live_browser_scroll_refresh.rs"]
mod refresh;
#[path = "live_browser_scroll_settle.rs"]
mod settle;
const LIMIT: usize = 128 * 1024 * 1024;
// 查询保持短预算；实测 Scroll 可在约两秒后成功，不能把 UIA transaction 设置当硬上限。
const QUERY_TIMEOUT_MS: u32 = 750;
const ACTION_TIMEOUT_MS: u32 = 3000;
pub(crate) const TIMEOUT_MS: u32 = QUERY_TIMEOUT_MS + ACTION_TIMEOUT_MS;
pub(crate) const STARTUP_TIMEOUT_MS: u32 = 2000;
static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(4 * LIMIT));
pub(crate) struct Worker {
    pipe: Pipe,
    child: bounded::Child,
    target: Target,
    residual: [i32; 2],
}
pub(crate) fn eligible(target: &Target) -> bool {
    let mut class = [0u16; 64];
    let n = unsafe { GetClassNameW(target.hwnd(), &mut class) } as usize;
    class[..n]
        .iter()
        .copied()
        .eq("Chrome_WidgetWin_1".encode_utf16())
}
impl Worker {
    pub(crate) fn start(target: Target) -> io::Result<Self> {
        Self::start_inner(target).map_err(start_error)
    }
    fn start_inner(target: Target) -> io::Result<Self> {
        target.validate().map_err(io::Error::other)?;
        let deadline = Instant::now() + Duration::from_millis(u64::from(STARTUP_TIMEOUT_MS));
        let (mut pipe, endpoint) = Pipe::pair()?;
        let child = bounded::start(
            endpoint.into_handle(),
            deadline,
            &|| false,
            bounded::Policy {
                suffix: if cfg!(test) {
                    " --exact live_browser_scroll::tests::child_entry --ignored --nocapture"
                } else {
                    " --native-browser-scroll-worker"
                },
                maximum: LIMIT,
                pool: &CAPACITY,
                environment: bounded::Environment::SystemRoot,
            },
        )?;
        let mut channel = pipe.io(Some(deadline), Some(child.handle()));
        channel.write_all(b"OBS2")?;
        channel.write_all(&(target.hwnd().0 as usize as u64).to_le_bytes())?;
        channel.write_all(&target.process_id().to_le_bytes())?;
        let mut ready = [0; 5];
        channel.read_exact(&mut ready)?;
        if ready[0] != 1 {
            return Err(response_error("initialization", ready));
        }
        Ok(Self {
            pipe,
            child,
            target,
            residual: [0; 2],
        })
    }
    pub(crate) fn scroll(
        &mut self,
        point: [i32; 2],
        delta: i32,
        horizontal: bool,
    ) -> io::Result<Option<i32>> {
        self.target.validate().map_err(io::Error::other)?;
        let ticks = accumulate(&mut self.residual[usize::from(horizontal)], delta)?;
        // 高分辨率滚轮保留不足一格的余量，不同时送 Win32，避免后续重复滚动。
        if ticks == 0 {
            return Ok(None);
        }
        let mut request = [0u8; 13];
        request[..4].copy_from_slice(&point[0].to_le_bytes());
        request[4..8].copy_from_slice(&point[1].to_le_bytes());
        request[8..12].copy_from_slice(&ticks.to_le_bytes());
        request[12] = u8::from(horizontal);
        let reply = protocol::request(&mut self.pipe, Some(self.child.handle()), &request)?;
        match reply[0] {
            0 => Ok(Some(ticks * 120)), // 连同先前累计量回退；余量仍保留，不能丢失小 delta。
            1 => Ok(None),
            _ => Err(response_error("operation", reply)),
        }
    }
}
fn start_error(error: io::Error) -> io::Error {
    // 启动握手或查询等待阶段尚未授权动作，可退休 child 后等待新用户输入。
    // 从尝试发送 commit 起不再使用此转换，管道超时必须继续作为未知结果失败。
    if error.kind() == io::ErrorKind::TimedOut {
        io::Error::new(io::ErrorKind::WouldBlock, error)
    } else {
        error
    }
}
fn response_error(stage: &str, reply: [u8; 5]) -> io::Error {
    let code = i32::from_le_bytes(reply[1..].try_into().unwrap());
    let kind = if reply[0] == 3 && process::transient(code) {
        io::ErrorKind::WouldBlock
    } else {
        io::ErrorKind::Other
    };
    io::Error::new(
        kind,
        format!("Browser scroll {stage} failed: 0x{:08x}", code as u32),
    )
}
pub(crate) fn physical_point(
    window: windows::Win32::Foundation::HWND,
    screen: [i32; 2],
) -> io::Result<[i32; 2]> {
    use windows::Win32::{Foundation::POINT, UI::HiDpi::*};
    let _dpi = crate::live_window_target::input_coordinates::Dpi::enter(window)
        .map_err(|e| io::Error::other(format!("{e:?}")))?;
    let mut point = POINT {
        x: screen[0],
        y: screen[1],
    };
    unsafe {
        if GetAwarenessFromDpiAwarenessContext(GetWindowDpiAwarenessContext(window))
            != DPI_AWARENESS_PER_MONITOR_AWARE
            && !LogicalToPhysicalPointForPerMonitorDPI(Some(window), &mut point).as_bool()
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok([point.x, point.y])
}
fn accumulate(residual: &mut i32, delta: i32) -> io::Result<i32> {
    if delta == 0 || delta.unsigned_abs() > 1200 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid browser wheel delta",
        ));
    }
    let total = *residual + delta;
    *residual = total % 120;
    Ok(total / 120)
}
#[cfg(test)]
#[path = "live_browser_scroll_tests.rs"]
mod tests;
