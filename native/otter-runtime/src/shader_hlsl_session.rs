//! Short-lived warm-reuse HLSL sessions bound to the same source/node: retain the 256MiB Job, per-request deadlines, 15s idle self-exit, and fixed-count rotation.
use super::{
    process::{Failure, CAPACITY, TIMEOUT},
    *,
};
use crate::{
    bounded_worker_process::{self as bounded, session as boundary},
    live_recovery_pipe::Pipe,
};
use std::{
    ffi::OsString,
    io::{self, Read},
    os::windows::io::{AsHandle, AsRawHandle},
    sync::{Condvar, LazyLock, Mutex},
    time::Duration,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_TIMEOUT},
    System::Threading::WaitForSingleObject,
};

pub const FLAG: &str = "--native-hlsl-session-worker";
const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
// The parent counts normal idle time after fully receiving the response; the child starts waiting earlier and provides only a longer lost-connection fallback.
const CHILD_IDLE_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_REQUESTS: usize = 32;
const MAX_PARKED_SCOPES: usize = 2;

struct BoundaryReader<R> {
    inner: R,
    started: bool,
}
impl<R: Read> Read for BoundaryReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buf)?;
        if count > 0 {
            self.started = true;
        }
        Ok(count)
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
    run_with_idle(CHILD_IDLE_TIMEOUT)
}

fn run_with_idle(idle: Duration) -> io::Result<()> {
    bounded::require_limits(CHILD_LIMIT)?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut engine: Option<super::WarmEngine> = None;
    for _ in 0..MAX_REQUESTS {
        let idle_deadline = Instant::now() + idle;
        let request = {
            let mut reader = BoundaryReader {
                inner: pipe.io(Some(idle_deadline), None),
                started: false,
            };
            match wire::read_request(&mut reader) {
                Ok(request) => request,
                Err(error) if !reader.started && boundary::idle_end(&error) => return Ok(()),
                Err(error) => return Err(error),
            }
        };
        let deadline = Instant::now() + TIMEOUT;
        let prepared = request.with_request(|value| {
            super::WarmEngine::prepare(&mut engine, value, request.backend, &|| false, deadline)
        });
        // Release request input pixels before allocating the CPU readback buffer to preserve the 4K 256MiB peak boundary.
        drop(request);
        let fatal = matches!(
            &prepared,
            Err(Error::Api { .. } | Error::TimedOut | Error::Cancelled | Error::Isolation)
        );
        let result = prepared.and_then(|ready| ready.finish(&|| false, deadline));
        let fatal = fatal
            || matches!(
                &result,
                Err(Error::Api { .. } | Error::TimedOut | Error::Cancelled | Error::Isolation)
            );
        let mut writer = pipe.io(Some(deadline), None);
        wire::write_response(&mut writer, &result)?;
        if fatal {
            return Ok(());
        }
    }
    Ok(())
}

struct Parked {
    scope: String,
    backend: Backend,
    suffix: &'static str,
    pipe: Pipe,
    child: bounded::Child,
    requests: usize,
    expires_at: Instant,
}
impl Parked {
    fn alive(&self) -> bool {
        unsafe {
            WaitForSingleObject(HANDLE(self.child.handle().as_raw_handle()), 0) == WAIT_TIMEOUT
        }
    }
}

#[derive(Default)]
struct PoolState {
    entries: Vec<Parked>,
    reaper_running: bool,
}

static POOL: LazyLock<(Mutex<PoolState>, Condvar)> =
    LazyLock::new(|| (Mutex::new(PoolState::default()), Condvar::new()));

pub fn render_scoped(
    scope: &str,
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
) -> std::result::Result<Frame, Failure> {
    render_with_entry(
        scope,
        request,
        backend,
        stopped,
        IDLE_TIMEOUT,
        " --native-hlsl-session-worker",
    )
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn render_scoped_in_test_harness(
    scope: &str,
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
) -> std::result::Result<Frame, Failure> {
    render_with_entry(
        scope,
        request,
        backend,
        stopped,
        IDLE_TIMEOUT,
        " --exact runtime_worker_entries::hlsl_session_worker_entry --ignored --nocapture",
    )
}

fn render_with_entry(
    scope: &str,
    request: &Request<'_>,
    backend: Backend,
    stopped: &impl Fn() -> bool,
    idle: Duration,
    suffix: &'static str,
) -> std::result::Result<Frame, Failure> {
    if scope.is_empty() || scope.len() > 1024 || scope.chars().any(char::is_control) {
        return Err(Failure::Backend(Error::Layout));
    }
    let deadline = Instant::now() + TIMEOUT;
    check(stopped, deadline).map_err(Failure::Backend)?;
    validate(request).map_err(Failure::Backend)?;
    let mut session = take_matching(scope, backend, suffix);
    if session.is_none() {
        let (pipe, endpoint) = Pipe::pair()?;
        let child = bounded::start_reclaiming(
            endpoint.into_handle(),
            deadline,
            stopped,
            bounded::Policy {
                suffix,
                maximum: CHILD_LIMIT,
                pool: &CAPACITY,
                environment: bounded::Environment::SystemRoot,
            },
            &reclaim_idle,
        )?;
        session = Some(Parked {
            scope: scope.to_owned(),
            backend,
            suffix,
            pipe,
            child,
            requests: 0,
            expires_at: Instant::now() + idle,
        });
    }
    let mut active = session.unwrap();
    let outcome = {
        let mut channel = active
            .pipe
            .cancellable_io(deadline, active.child.handle(), stopped);
        wire::write_request(&mut channel, request, backend)
            .and_then(|()| wire::read_response(&mut channel, [request.width, request.height]))
    };
    let result = match outcome {
        Ok(response) => response,
        Err(io_err) => {
            drop(active);
            return Err(Failure::Transport(io_err));
        }
    };
    check(stopped, deadline).map_err(Failure::Backend)?;
    let keep = match &result {
        Ok(_) => true,
        Err(Error::Compile(_) | Error::Layout | Error::Budget | Error::Unsupported) => true,
        Err(_) => false,
    };
    active.requests += 1;
    if keep && active.requests < MAX_REQUESTS {
        boundary::idle(&active.pipe, &active.child)?;
        active.expires_at = Instant::now() + idle;
        park(active);
    } else {
        boundary::finish(&mut active.pipe, &active.child, deadline, stopped)?;
        drop(active);
    }
    check(stopped, deadline).map_err(Failure::Backend)?;
    result.map_err(Failure::Backend)
}

fn take_matching(scope: &str, backend: Backend, suffix: &'static str) -> Option<Parked> {
    let now = Instant::now();
    let (mut expired, mut found) = (Vec::new(), None);
    if let Ok(mut state) = POOL.0.lock() {
        let mut i = 0;
        while i < state.entries.len() {
            if state.entries[i].expires_at <= now || !state.entries[i].alive() {
                expired.push(state.entries.swap_remove(i));
            } else if found.is_none()
                && state.entries[i].scope == scope
                && state.entries[i].backend == backend
                && state.entries[i].suffix == suffix
            {
                found = Some(state.entries.swap_remove(i));
            } else {
                i += 1;
            }
        }
    }
    drop(expired);
    // Before sending, retire workers that failed while idle; never replay already-sent requests.
    found.filter(|entry| boundary::idle(&entry.pipe, &entry.child).is_ok())
}

/// Only parked entries within the pool may be reclaimed; active entries have been removed, and capacity waiters recheck each round.
pub(super) fn reclaim_idle() {
    let victim = POOL.0.lock().ok().and_then(|mut state| {
        if state.entries.len() >= MAX_PARKED_SCOPES || CAPACITY.used() >= CHILD_LIMIT {
            state
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.expires_at)
                .map(|(idx, _)| idx)
                .map(|idx| state.entries.swap_remove(idx))
        } else {
            None
        }
    });
    drop(victim);
}

fn park(entry: Parked) {
    park_with(entry, || {
        std::thread::Builder::new()
            .name("otter-hlsl-idle-reap".into())
            .spawn(reap_loop)
            .map(|_| ())
    });
}

fn park_with(entry: Parked, spawn: impl FnOnce() -> io::Result<()>) {
    let mut spawn_reaper = false;
    let mut evicted = Vec::new();
    if let Ok(mut state) = POOL.0.lock() {
        while state.entries.len() >= MAX_PARKED_SCOPES {
            if let Some((idx, _)) = state
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, e)| e.expires_at)
            {
                evicted.push(state.entries.swap_remove(idx));
            }
        }
        state.entries.push(entry);
        if !state.reaper_running {
            state.reaper_running = true;
            spawn_reaper = true;
        }
        POOL.1.notify_all();
    }
    drop(evicted);
    if spawn_reaper && spawn().is_err() {
        // running belongs to this reservation; other park calls do not start threads while failure is handled.
        // Return the reservation and take orphaned entries under the lock, then destroy outside it; later park calls may then reserve a new reaper.
        let orphaned = {
            let mut state = POOL.0.lock().unwrap_or_else(|e| e.into_inner());
            state.reaper_running = false;
            std::mem::take(&mut state.entries)
        };
        drop(orphaned);
    }
}

fn reap_loop() {
    loop {
        let mut expired = Vec::new();
        let wait = {
            let mut guard = POOL.0.lock().unwrap_or_else(|e| e.into_inner());
            let now = Instant::now();
            let mut i = 0;
            while i < guard.entries.len() {
                if guard.entries[i].expires_at <= now || !guard.entries[i].alive() {
                    expired.push(guard.entries.swap_remove(i));
                } else {
                    i += 1;
                }
            }
            if expired.is_empty() {
                if let Some(next) = guard.entries.iter().map(|e| e.expires_at).min() {
                    let delay = next.saturating_duration_since(now);
                    let _ = POOL.1.wait_timeout(guard, delay);
                    None
                } else {
                    guard.reaper_running = false;
                    return;
                }
            } else {
                Some(())
            }
        };
        drop(expired);
        let _ = wait;
    }
}

#[cfg(test)]
#[path = "shader_hlsl_session_tests.rs"]
mod tests;
