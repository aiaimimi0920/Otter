//! 已认证同EXE独立紧急退出child；不读profile、不注入输入、不承担Live恢复责任。
use crate::{
    emergency_exit::Tracker, live_recovery_handshake, live_recovery_parent::Parent,
    live_recovery_pipe::Pipe,
};
use std::{
    ffi::OsString,
    io,
    os::windows::io::{AsHandle, AsRawHandle},
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, System::Threading::*, UI::Input::KeyboardAndMouse::*};

#[path = "emergency_watchdog_client.rs"]
mod client;
pub use client::Owner;
#[cfg(test)]
#[path = "emergency_watchdog_tests.rs"]
mod tests;

pub const FLAG: &str = "--native-emergency-parent";
const BUDGET: Duration = Duration::from_secs(5);
const SAMPLE_MS: u32 = 8;

pub fn parse(args: &[OsString]) -> io::Result<Option<u32>> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(None);
    }
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid native emergency invocation",
        )
    };
    if args.len() != 2 || args[0] != FLAG {
        return Err(invalid());
    }
    let text = args[1].to_str().ok_or_else(invalid)?;
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    let pid = text.parse::<u32>().map_err(|_| invalid())?;
    if pid == 0 {
        return Err(invalid());
    }
    Ok(Some(pid))
}
pub fn install() -> Option<Owner> {
    match Owner::start() {
        Ok(owner) => Some(owner),
        Err(error) => {
            eprintln!("native-emergency-watchdog: unavailable: {error}");
            None
        }
    }
}
pub fn run_inherited(parent: u32) -> io::Result<()> {
    let pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    run(parent, pipe, physical)
}
#[derive(Default, Clone, Copy)]
struct Keys {
    escape: bool,
    delete: bool,
    backup: bool,
}
fn physical() -> Keys {
    let down = |key: VIRTUAL_KEY| unsafe { GetAsyncKeyState(i32::from(key.0)) < 0 };
    Keys {
        escape: down(VK_ESCAPE),
        delete: down(VK_DELETE),
        backup: down(VK_CONTROL) && down(VK_MENU) && down(VK_SHIFT) && down(VK_F12),
    }
}
pub(crate) fn observe_message(tracker: &mut Tracker, message: u32, key: usize) -> bool {
    tracker.message(message, key, Instant::now(), || {
        let keys = physical();
        (keys.escape, keys.delete)
    })
}
#[derive(Default)]
struct Gate {
    armed: bool,
    backup: bool,
    tracker: Tracker,
}
impl Gate {
    fn observe(&mut self, keys: Keys, now: Instant) -> bool {
        // 启动时已按住的键不能立即触发；先观测完整松开，不记录按键文本/历史。
        if !self.armed {
            self.armed = !keys.escape && !keys.delete && !keys.backup;
            return false;
        }
        let backup = keys.backup && !self.backup;
        self.backup = keys.backup;
        self.tracker.state(keys.escape, keys.delete, now) || backup
    }
}
fn run(parent: u32, mut pipe: Pipe, mut keys: impl FnMut() -> Keys) -> io::Result<()> {
    let parent = Parent::authenticate_emergency(parent, pipe.handle()).map_err(io::Error::other)?;
    let mut channel = pipe.io(Some(Instant::now() + BUDGET), Some(parent.handle()));
    let nonce = live_recovery_handshake::read_hello(&mut channel)?;
    live_recovery_handshake::write_ready(&mut channel, nonce)?;
    let mut gate = Gate::default();
    loop {
        // 旧产品同为8ms采样；独立进程不依赖UI消息泵、模态或挂起状态。
        match unsafe { WaitForSingleObject(HANDLE(parent.handle().as_raw_handle()), SAMPLE_MS) } {
            WAIT_OBJECT_0 => return Ok(()),
            WAIT_TIMEOUT => {}
            WAIT_FAILED => return Err(io::Error::last_os_error()),
            _ => return Err(io::Error::other("Unexpected emergency parent wait status")),
        }
        if gate.observe(keys(), Instant::now()) {
            parent.terminate_emergency().map_err(io::Error::other)?;
            return Ok(());
        }
    }
}
