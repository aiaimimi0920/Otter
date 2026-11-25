//! 用户完成保存/退出确认后才给已有 watchdog 设截止；不杀恢复源窗口的 child。
use super::Owner;
#[cfg(test)]
use super::BUDGET;
use crate::live_recovery_pipe::Pipe;
#[cfg(test)]
use std::io::Read;
use std::{
    cell::RefCell,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    time::{Duration, Instant},
};
#[cfg(test)]
use windows::Win32::System::Pipes::PeekNamedPipe;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{ERROR_IO_PENDING, HANDLE, WAIT_OBJECT_0},
        Storage::FileSystem::ReadFile,
        System::{
            Threading::{CreateEventW, ResetEvent, WaitForSingleObject},
            IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        },
    },
};

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
    #[cfg(test)]
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
    #[cfg(test)]
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

/// 生产 watchdog 的事件驱动关闭监听：overlapped 单字节读取，空闲时零轮询，收到或断开后停读。
pub(crate) struct Listener<'a> {
    pipe: HANDLE,
    // 保证 Drop 等待取消完成之前原管道句柄不会关闭或被系统复用。
    _owner: &'a Pipe,
    overlapped: Box<OVERLAPPED>,
    byte: Box<[u8; 1]>,
    active: bool,
    deadline: Deadline,
    event: OwnedHandle,
}
impl<'a> Listener<'a> {
    pub(crate) fn new(pipe: &'a Pipe) -> io::Result<Self> {
        let raw = unsafe { CreateEventW(None, true, false, None) }.map_err(io::Error::other)?;
        let event = unsafe { OwnedHandle::from_raw_handle(raw.0) };
        let mut listener = Self {
            pipe: HANDLE(pipe.handle().as_raw_handle()),
            _owner: pipe,
            overlapped: Box::new(OVERLAPPED {
                hEvent: HANDLE(event.as_raw_handle()),
                ..Default::default()
            }),
            byte: Box::new([0u8; 1]),
            active: false,
            deadline: Deadline::default(),
            event,
        };
        listener.arm(Instant::now());
        Ok(listener)
    }
    pub(crate) fn wait_handle(&self) -> Option<HANDLE> {
        self.active.then(|| HANDLE(self.event.as_raw_handle()))
    }
    pub(crate) fn remaining(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .at
            .map(|end| end.saturating_duration_since(now))
    }
    pub(crate) fn expired(&self, now: Instant) -> bool {
        self.deadline.expired(now)
    }
    pub(crate) fn poll_signaled(&mut self, now: Instant) {
        if self.active
            && unsafe { WaitForSingleObject(HANDLE(self.event.as_raw_handle()), 0) }
                == WAIT_OBJECT_0
        {
            self.active = false;
            self.finish_read(now);
            self.arm(now);
        }
    }
    fn arm(&mut self, now: Instant) {
        while !self.active && !self.deadline.channel_failed && self.deadline.at.is_none() {
            if unsafe { ResetEvent(HANDLE(self.event.as_raw_handle())) }.is_err() {
                self.deadline.channel_failed = true;
                break;
            }
            match unsafe {
                ReadFile(
                    self.pipe,
                    Some(&mut self.byte[..]),
                    None,
                    Some(&mut *self.overlapped),
                )
            } {
                Ok(()) => self.finish_read(now),
                Err(e) if e.code() == HRESULT::from_win32(ERROR_IO_PENDING.0) => {
                    self.active = true;
                    break;
                }
                Err(error) => {
                    self.deadline.channel_failed = true;
                    eprintln!("native-shutdown-watchdog: command channel failed: {error}");
                    break;
                }
            }
        }
    }
    fn finish_read(&mut self, now: Instant) {
        let mut transferred = 0;
        match unsafe { GetOverlappedResult(self.pipe, &*self.overlapped, &mut transferred, false) }
        {
            Ok(()) if transferred == 1 => {
                if let Err(error) = self.deadline.accept(self.byte[0], now) {
                    self.deadline.channel_failed = true;
                    eprintln!("native-shutdown-watchdog: command channel failed: {error}");
                }
            }
            Ok(()) => self.deadline.channel_failed = true,
            Err(error) => {
                self.deadline.channel_failed = true;
                eprintln!("native-shutdown-watchdog: command channel failed: {error}");
            }
        }
    }
}
impl Drop for Listener<'_> {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                let _ = CancelIoEx(self.pipe, Some(&*self.overlapped));
                let mut count = 0;
                let _ = GetOverlappedResult(self.pipe, &*self.overlapped, &mut count, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use windows::Win32::Foundation::WAIT_TIMEOUT;
    #[test]
    fn pending_listener_drop_cancels_before_pipe_can_be_reused() {
        let (mut pipe, mut peer) = Pipe::pair().unwrap();
        drop(Listener::new(&pipe).unwrap());
        peer.io(Some(Instant::now() + BUDGET), None)
            .write_all(&[COMMAND])
            .unwrap();
        let mut byte = [0];
        pipe.io(Some(Instant::now() + BUDGET), None)
            .read_exact(&mut byte)
            .unwrap();
        assert_eq!(byte, [COMMAND]);
    }
    #[test]
    fn overlapped_listener_sleeps_idle_and_retires_handle_on_command_or_eof() {
        let (pipe, mut peer) = Pipe::pair().unwrap();
        let mut listener = Listener::new(&pipe).unwrap();
        let handle = listener.wait_handle().expect("armed overlapped read");
        assert_eq!(unsafe { WaitForSingleObject(handle, 0) }, WAIT_TIMEOUT);
        peer.io(Some(Instant::now() + BUDGET), None)
            .write_all(&[COMMAND])
            .unwrap();
        assert_eq!(unsafe { WaitForSingleObject(handle, 1000) }, WAIT_OBJECT_0);
        let now = Instant::now();
        listener.poll_signaled(now);
        assert!(listener.wait_handle().is_none());
        assert!(listener.remaining(now).is_some());
        assert!(listener.expired(now + GRACE));
        drop(listener);

        let (pipe2, peer2) = Pipe::pair().unwrap();
        let mut listener2 = Listener::new(&pipe2).unwrap();
        let handle2 = listener2.wait_handle().unwrap();
        drop(peer2);
        assert_eq!(unsafe { WaitForSingleObject(handle2, 1000) }, WAIT_OBJECT_0);
        listener2.poll_signaled(Instant::now());
        assert!(listener2.wait_handle().is_none());
    }
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
