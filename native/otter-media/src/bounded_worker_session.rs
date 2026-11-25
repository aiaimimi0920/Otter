//! 热 worker 的帧间边界：拒绝已知退出/尾随输出，轮转退出仍核验 exit code 与 EOF。
use super::Child;
use crate::live_recovery_pipe::Pipe;
use std::{
    io::{self, Read},
    os::windows::io::AsRawHandle,
    time::Instant,
};
use windows::Win32::{
    Foundation::{
        ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED, HANDLE, WAIT_FAILED,
        WAIT_TIMEOUT,
    },
    System::{Pipes::PeekNamedPipe, Threading::WaitForSingleObject},
};

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Worker session boundary failed")
}

/// 不读取/丢弃字节。只能证明当前边界干净，不承诺仍在运行的进程今后不会失败。
pub fn idle(pipe: &Pipe, child: &Child) -> io::Result<()> {
    quiet(pipe)?;
    match unsafe { WaitForSingleObject(HANDLE(child.handle().as_raw_handle()), 0) } {
        WAIT_TIMEOUT => Ok(()),
        WAIT_FAILED => Err(io::Error::last_os_error()),
        _ => Err(invalid()),
    }
}

fn quiet(pipe: &Pipe) -> io::Result<()> {
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
    if available == 0 {
        Ok(())
    } else {
        Err(invalid())
    }
}

pub fn finish(
    pipe: &mut Pipe,
    child: &Child,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
) -> io::Result<()> {
    child.finish(deadline, stopped)?;
    let mut extra = [0];
    match pipe.io(Some(deadline), None).read(&mut extra) {
        Ok(0) => Ok(()),
        Err(error) if closed(&error) => Ok(()),
        _ => Err(invalid()),
    }
}

pub fn idle_end(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::UnexpectedEof | io::ErrorKind::TimedOut
    ) || closed(error)
}

fn closed(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::BrokenPipe
        || error
            .get_ref()
            .and_then(|e| e.downcast_ref::<windows::core::Error>())
            .is_some_and(|e| {
                [ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED]
                    .iter()
                    .any(|code| e.code() == windows::core::HRESULT::from_win32(code.0))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, os::windows::io::AsHandle, time::Duration};
    #[test]
    fn queued_output_and_unexpected_idle_errors_are_rejected() {
        let (mut pipe, mut peer) = Pipe::pair().unwrap();
        quiet(&pipe).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        peer.io(Some(deadline), None).write_all(&[9]).unwrap();
        assert!(quiet(&pipe).is_err());
        pipe.io(Some(deadline), None).read_exact(&mut [0]).unwrap();
        quiet(&pipe).unwrap();
        assert!(!idle_end(&io::Error::other("unexpected I/O failure")));
        assert!(idle_end(&io::ErrorKind::TimedOut.into()));
        drop(peer);
        assert!(quiet(&pipe).is_err());
    }

    #[test]
    #[ignore = "private bounded session boundary fixture"]
    fn boundary_child() {
        let mut pipe =
            Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned().unwrap()).unwrap();
        let mut channel = pipe.io(Some(Instant::now() + Duration::from_secs(5)), None);
        let mut mode = [0];
        channel.read_exact(&mut mode).unwrap();
        channel
            .write_all(if mode[0] == 2 { &[7, 9] } else { &[7] })
            .unwrap();
        drop(channel);
        drop(pipe);
        std::process::exit(if mode[0] == 1 { 1 } else { 0 });
    }

    #[test]
    fn terminal_boundary_requires_successful_exit_and_no_extra_output() {
        let pool = crate::byte_budget::Budget::new(128 * 1024 * 1024);
        for mode in [0, 1, 2] {
            let deadline = Instant::now() + Duration::from_secs(10);
            let (mut pipe, endpoint) = Pipe::pair().unwrap();
            let child = super::super::start(endpoint.into_handle(), deadline, &|| false,
                super::super::Policy {
                    suffix: " --exact bounded_worker_process::session::tests::boundary_child --ignored --nocapture",
                    maximum: 128 * 1024 * 1024,
                    pool: &pool,
                    environment: super::super::Environment::SystemRoot,
                }).unwrap();
            let mut channel = pipe.cancellable_io(deadline, child.handle(), &|| false);
            channel.write_all(&[mode]).unwrap();
            let mut frame = [0];
            channel.read_exact(&mut frame).unwrap();
            assert_eq!(frame, [7]);
            drop(channel);
            assert_eq!(
                finish(&mut pipe, &child, deadline, &|| false).is_ok(),
                mode == 0
            );
            assert!(idle(&pipe, &child).is_err());
            drop(child);
            assert_eq!(pool.used(), 0);
        }
    }
}
