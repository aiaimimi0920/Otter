//! Query/execution confirmation boundary: no action before the parent's commit arrives, and no replay of unknown action outcomes.
use super::*;
use std::os::windows::io::BorrowedHandle;
const PREPARED: u8 = 4;
const COMMIT: u8 = 1;

pub(super) fn request(
    pipe: &mut Pipe,
    child: Option<BorrowedHandle<'_>>,
    request: &[u8; 13],
) -> io::Result<[u8; 5]> {
    request_with_budgets(pipe, child, request, QUERY_TIMEOUT_MS, ACTION_TIMEOUT_MS)
}
fn request_with_budgets(
    pipe: &mut Pipe,
    child: Option<BorrowedHandle<'_>>,
    request: &[u8; 13],
    query_ms: u32,
    action_ms: u32,
) -> io::Result<[u8; 5]> {
    let mut reply = [0; 5];
    // No action is authorized in this phase. Even if the child's query RPC hangs, it can be retired safely.
    (|| -> io::Result<()> {
        let mut channel = pipe.io(Some(after(query_ms)), child);
        channel.write_all(request)?;
        channel.read_exact(&mut reply)
    })()
    .map_err(start_error)?;
    match reply[0] {
        0 | 2 | 3 => return Ok(reply),
        PREPARED if reply[1..] == [0; 4] => {}
        _ => return Err(io::Error::other("Invalid browser scroll prepare reply")),
    }
    let mut channel = pipe.io(Some(after(action_ms)), child);
    // Once writing commit is attempted, failure means an unknown outcome and is no longer converted to a query transition.
    channel.write_all(&[COMMIT])?;
    channel.read_exact(&mut reply)?;
    if !matches!(reply[0], 1..=3) {
        return Err(io::Error::other("Invalid browser scroll action reply"));
    }
    Ok(reply)
}
pub(super) fn respond<T>(
    pipe: &mut Pipe,
    query_deadline: Instant,
    prepare: impl FnOnce() -> windows::core::Result<Option<T>>,
    apply: impl FnOnce(T, Instant, &mut bool) -> windows::core::Result<bool>,
) -> io::Result<()> {
    let prepared = match prepare() {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return finish(pipe, query_deadline, Ok(false), false),
        Err(error) => return finish(pipe, query_deadline, Err(error), false),
    };
    pipe.io(Some(query_deadline), None)
        .write_all(&[PREPARED, 0, 0, 0, 0])?;
    // Sending Prepared enters the action phase; commit waiting also consumes its budget, without inheriting the query deadline.
    let deadline = after(ACTION_TIMEOUT_MS - 50);
    let mut commit = [0];
    pipe.io(Some(deadline), None).read_exact(&mut commit)?;
    if commit != [COMMIT] {
        return Err(io::Error::other("Invalid browser scroll commit"));
    }
    let mut attempted = false;
    let result = apply(prepared, deadline, &mut attempted);
    finish(pipe, deadline, result, attempted)
}
fn finish(
    pipe: &mut Pipe,
    deadline: Instant,
    result: windows::core::Result<bool>,
    attempted: bool,
) -> io::Result<()> {
    let reply = match result {
        Ok(handled) => [u8::from(handled), 0, 0, 0, 0],
        Err(error) => process::failure_reply(&error, attempted),
    };
    pipe.io(Some(deadline), None).write_all(&reply)?;
    if reply[0] == 2 {
        return Err(io::Error::other("Browser scroll operation failed"));
    }
    Ok(())
}
fn after(ms: u32) -> Instant {
    Instant::now() + Duration::from_millis(u64::from(ms))
}

#[cfg(test)]
#[path = "live_browser_scroll_protocol_tests.rs"]
mod tests;
