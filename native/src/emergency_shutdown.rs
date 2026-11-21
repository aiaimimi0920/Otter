//! 用户完成保存/退出确认后才给已有 watchdog 设截止；不杀恢复源窗口的 child。
use super::{Owner, BUDGET};
use crate::live_recovery_pipe::Pipe;
use std::{
    cell::RefCell,
    io::{self, Read},
    os::windows::io::AsRawHandle,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::HANDLE, System::Pipes::PeekNamedPipe};

const COMMAND: u8 = 0x53;
const GRACE: Duration = Duration::from_secs(30);
thread_local! { static OWNER: RefCell<Option<Owner>> = const { RefCell::new(None) }; }

/// 归 main 栈所有；正常退出仍析构并等待唯一自建 watchdog。
pub struct Installed(std::marker::PhantomData<std::rc::Rc<()>>);
impl Drop for Installed {
    fn drop(&mut self) {
        OWNER.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
pub(super) fn install(owner: Owner) -> Installed {
    OWNER.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(owner);
    });
    Installed(std::marker::PhantomData)
}
pub fn begin() {
    OWNER.with(|slot| {
        if let Some(owner) = slot.borrow_mut().as_mut() {
            if let Err(error) = owner.begin_shutdown() {
                eprintln!("native-shutdown-watchdog: could not arm: {error}");
            }
        }
    });
}
pub(super) fn command() -> u8 {
    COMMAND
}

#[derive(Default)]
pub(super) struct Deadline {
    at: Option<Instant>,
    channel_failed: bool,
}
impl Deadline {
    fn accept(&mut self, command: u8, now: Instant) -> io::Result<()> {
        if command != COMMAND {
            return Err(io::Error::other("Invalid shutdown watchdog command"));
        }
        self.at.get_or_insert(now + GRACE);
        Ok(())
    }
    pub(super) fn poll(&mut self, pipe: &mut Pipe) -> bool {
        if !self.channel_failed {
            if let Err(error) = self.read_command(pipe) {
                // 管道故障不能撤销已确认的截止，也不能停掉原紧急热键监护。
                self.channel_failed = true;
                eprintln!("native-shutdown-watchdog: command channel failed: {error}");
            }
        }
        self.expired(Instant::now())
    }
    fn read_command(&mut self, pipe: &mut Pipe) -> io::Result<()> {
        let mut available = 0;
        unsafe {
            PeekNamedPipe(
                HANDLE(pipe.handle().as_raw_handle()),
                None,
                0,
                None,
                Some(&mut available),
                None,
            )
        }
        .map_err(io::Error::other)?;
        if available > 0 {
            let mut command = [0];
            pipe.io(Some(Instant::now() + BUDGET), None)
                .read_exact(&mut command)?;
            self.accept(command[0], Instant::now())?;
        }
        Ok(())
    }
    fn expired(&self, now: Instant) -> bool {
        self.at.is_some_and(|end| now >= end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broken_channel_preserves_armed_deadline_and_stops_repeated_reads() {
        let (mut pipe, peer) = Pipe::pair().unwrap();
        let now = Instant::now();
        let mut deadline = Deadline::default();
        deadline.accept(COMMAND, now).unwrap();
        drop(peer);
        assert!(!deadline.poll(&mut pipe));
        assert!(deadline.channel_failed);
        assert!(deadline.expired(now + GRACE));
    }
    #[test]
    fn only_explicit_commit_arms_and_repeated_commands_do_not_extend_exit() {
        let now = Instant::now();
        let mut deadline = Deadline::default();
        assert!(!deadline.expired(now + GRACE * 2));
        assert!(deadline.accept(0, now).is_err());
        assert!(!deadline.expired(now + GRACE * 2));
        deadline.accept(COMMAND, now).unwrap();
        deadline
            .accept(COMMAND, now + Duration::from_secs(20))
            .unwrap();
        assert!(!deadline.expired(now + GRACE - Duration::from_millis(1)));
        assert!(deadline.expired(now + GRACE));
    }
}
