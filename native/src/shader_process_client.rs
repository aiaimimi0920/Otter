//! 父侧发布门：单一绝对期限覆盖容量/传输/退出，完整帧、正常退出与 EOF 后才交付。
use super::*;
use crate::{bounded_worker_process::Child, live_recovery_pipe::Pipe, shader_worker_wire as wire};
use std::{io::Read, os::windows::io::OwnedHandle};
pub(super) fn run(
    invocation: Invocation<'_>,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    spawn: impl FnOnce(OwnedHandle, Instant) -> io::Result<Child>,
) -> Result<Frame, Failure> {
    if stopped() {
        return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Shader cancelled").into());
    }
    let header = wire::Header::for_invocation(&invocation)?;
    let (mut pipe, endpoint) = Pipe::pair()?;
    let child = spawn(endpoint.into_handle(), deadline)?;
    let result = {
        let mut channel = pipe.cancellable_io(deadline, child.handle(), stopped);
        wire::write_request(&mut channel, &invocation, &header)?;
        wire::read_response(&mut channel, [invocation.width, invocation.height])?
    };
    child.finish(deadline, stopped)?;
    let mut extra = [0];
    match pipe.io(Some(deadline), None).read(&mut extra) {
        Ok(0) => {}
        Err(error) if closed(&error) => {}
        _ => return Err(wire::invalid().into()),
    }
    if stopped() {
        return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Shader cancelled").into());
    }
    result.map_err(Failure::Backend)
}
fn closed(error: &io::Error) -> bool {
    use windows::Win32::Foundation::{ERROR_BROKEN_PIPE, ERROR_NO_DATA, ERROR_PIPE_NOT_CONNECTED};
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
