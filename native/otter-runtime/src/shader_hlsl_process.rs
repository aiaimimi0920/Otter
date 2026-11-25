//! Parent-side synchronous HLSL API, called only by existing background jobs; reuse the sole Job/pipe owner without touching drivers in the parent.
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
pub(super) const TIMEOUT: Duration = Duration::from_secs(15);
// MAIN integration selects only one Shader path; this pool adds no default workers/threads or prebuilt GPU owners.
pub(super) static CAPACITY: LazyLock<Arc<crate::byte_budget::Budget>> =
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
    // The single-request child's input is no longer borrowed by GPU; zero/return it first to avoid holding two complete 4K CPU images simultaneously.
    drop(request);
    let result = result.and_then(|prepared| prepared.finish(&|| false, deadline));
    wire::write_response(&mut channel, &result)
}
/// One absolute deadline covers admission/transport/normal exit/EOF; failure/cancellation must not publish successful frames.
pub fn render(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
) -> std::result::Result<Frame, Failure> {
    // Cargo can unify dev features into an integration-test build of the product EXE.
    // Only a caller that owns a libtest executable may select its private test entry.
    render_with_entry(request, backend, stopped, " --native-hlsl-worker")
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn render_in_test_harness(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
) -> std::result::Result<Frame, Failure> {
    render_with_entry(
        request,
        backend,
        stopped,
        " --exact runtime_worker_entries::hlsl_worker_entry --ignored --nocapture",
    )
}

pub(super) fn render_with_entry(
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    suffix: &'static str,
) -> std::result::Result<Frame, Failure> {
    run(
        request,
        backend,
        Instant::now() + TIMEOUT,
        stopped,
        |endpoint, deadline| {
            bounded::start_reclaiming(
                endpoint,
                deadline,
                stopped,
                bounded::Policy {
                    suffix,
                    maximum: CHILD_LIMIT,
                    pool: &CAPACITY,
                    environment: bounded::Environment::SystemRoot,
                },
                &super::session::reclaim_idle,
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
