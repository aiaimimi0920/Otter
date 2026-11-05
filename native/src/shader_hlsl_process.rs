//! HLSL 父侧同步 API，必须由既有后台任务调用；复用唯一 Job/pipe owner，不在父进程碰驱动。
use super::*;
use crate::{bounded_worker_process as bounded, live_recovery_pipe::Pipe};
use std::{
    ffi::OsString,
    io::{self, Read},
    os::windows::io::{AsHandle, OwnedHandle},
    sync::{Arc, LazyLock},
    time::Duration,
};
pub const FLAG: &str = "--native-hlsl-worker";
const TIMEOUT: Duration = Duration::from_secs(15);
// MAIN 集成时只选择一条 Shader 路径；此池不增加默认 worker/线程或预建 GPU owner。
static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
    LazyLock::new(|| crate::byte_budget::Budget::new(CHILD_LIMIT));
#[derive(Debug)]
pub enum Failure {
    Transport(io::Error),
    Backend(Error),
}
impl From<io::Error> for Failure {
    fn from(value: io::Error) -> Self {
        Self::Transport(value)
    }
}
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|a| a == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != FLAG {
        return Err(wire::invalid());
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    bounded::require_limits(CHILD_LIMIT)?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let deadline = Instant::now() + TIMEOUT;
    let mut channel = pipe.io(Some(deadline), None);
    let request = wire::read_request(&mut channel)?;
    let result =
        request.with_request(|value| super::prepare(value, request.backend, &|| false, deadline));
    // 单请求 child 的输入不再被 GPU 借用；先清零/归还它，避免 4K 同时持有两份 CPU 全图。
    drop(request);
    let result = result.and_then(|prepared| prepared.finish(&|| false, deadline));
    wire::write_response(&mut channel, &result)
}
/// 单一绝对期限覆盖准入/传输/正常退出与 EOF；失败/取消不能发布成功帧。
pub fn render(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
) -> std::result::Result<Frame, Failure> {
    run(
        request,
        backend,
        Instant::now() + TIMEOUT,
        stopped,
        |endpoint, deadline| {
            bounded::start(
                endpoint,
                deadline,
                stopped,
                bounded::Policy {
                    suffix: if cfg!(test) {
                        " --exact shader_hlsl::process::tests::hlsl_worker_entry --ignored --nocapture"
                    } else {
                        " --native-hlsl-worker"
                    },
                    maximum: CHILD_LIMIT,
                    pool: &CAPACITY,
                    environment: bounded::Environment::SystemRoot,
                },
            )
        },
    )
}
fn run(
    request: &Request<'_>,
    backend: Backend,
    deadline: Instant,
    stopped: &impl Fn() -> bool,
    spawn: impl FnOnce(OwnedHandle, Instant) -> io::Result<bounded::Child>,
) -> std::result::Result<Frame, Failure> {
    check(stopped, deadline).map_err(Failure::Backend)?;
    validate(request).map_err(Failure::Backend)?;
    let (mut pipe, endpoint) = Pipe::pair()?;
    let child = spawn(endpoint.into_handle(), deadline)?;
    let result = {
        let mut channel = pipe.cancellable_io(deadline, child.handle(), stopped);
        wire::write_request(&mut channel, request, backend)?;
        wire::read_response(&mut channel, [request.width, request.height])?
    };
    child.finish(deadline, stopped)?;
    let mut extra = [0];
    match pipe.io(Some(deadline), None).read(&mut extra) {
        Ok(0) => {}
        Err(error) if closed(&error) => {}
        _ => return Err(wire::invalid().into()),
    }
    check(stopped, deadline).map_err(Failure::Backend)?;
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
#[cfg(test)]
#[path = "shader_hlsl_process_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn render_observed(
    request: &Request<'_>,
    backend: Backend,
) -> (std::result::Result<Frame, Failure>, usize) {
    let mut probe = None;
    let result = run(
        request,
        backend,
        Instant::now() + TIMEOUT,
        &|| false,
        |endpoint, deadline| {
            let child = bounded::start(endpoint, deadline, &|| false, bounded::Policy {
            suffix: " --exact shader_hlsl::process::tests::hlsl_worker_entry --ignored --nocapture",
            maximum: CHILD_LIMIT, pool: &CAPACITY, environment: bounded::Environment::SystemRoot,
        })?;
            probe = Some(child.probe()?);
            Ok(child)
        },
    );
    let probe = probe.expect("observed renderer child started");
    assert!(probe.exited());
    let peak = probe.peak_job_bytes().unwrap();
    assert!(peak > 0 && peak <= CHILD_LIMIT);
    assert_eq!(CAPACITY.used(), 0);
    (result, peak)
}
