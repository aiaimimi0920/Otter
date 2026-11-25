//! Private scroll-child protocol; created only by the local input owner, without reusing read-only observation permissions or remote protocols.
use super::*;
use std::{ffi::OsString, os::windows::io::AsHandle};
use windows::Win32::Foundation::HWND;
pub const FLAG: &str = "--native-browser-scroll-worker";
pub fn parse(args: &[OsString]) -> io::Result<bool> {
    if !args.iter().any(|a| a == FLAG) {
        return Ok(false);
    }
    if args.len() != 1 || args[0] != FLAG {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid browser scroll worker arguments",
        ));
    }
    Ok(true)
}
pub fn run_inherited() -> io::Result<()> {
    bounded::require_limits(LIMIT)?;
    let mut pipe = Pipe::adopt(std::io::stdin().as_handle().try_clone_to_owned()?)?;
    let mut header = [0u8; 16];
    pipe.io(Some(Instant::now() + Duration::from_secs(2)), None)
        .read_exact(&mut header)?;
    if &header[..4] != b"OBS2" {
        return Err(io::Error::other("Invalid browser scroll header"));
    }
    let raw = usize::try_from(u64::from_le_bytes(header[4..12].try_into().unwrap()))
        .map_err(io::Error::other)?;
    let target = Target::bind(HWND(raw as *mut _)).map_err(io::Error::other)?;
    if target.process_id() != u32::from_le_bytes(header[12..].try_into().unwrap())
        || !eligible(&target)
    {
        return Err(io::Error::other("Browser scroll source identity rejected"));
    }
    crate::live_window_target::input_security::preflight(&target)
        .map_err(|e| io::Error::other(format!("{e:?}")))?;
    let initialized = provider::Provider::new(target);
    let mut ready = [0u8; 5];
    match &initialized {
        Ok(_) => ready[0] = 1,
        Err(error) => ready = failure_reply(error, false),
    }
    pipe.io(Some(Instant::now() + Duration::from_secs(2)), None)
        .write_all(&ready)?;
    let provider = initialized.map_err(io::Error::other)?;
    loop {
        let mut request = [0u8; 13];
        pipe.io(None, None).read_exact(&mut request[..1])?;
        let deadline = Instant::now() + Duration::from_millis(u64::from(QUERY_TIMEOUT_MS - 50));
        pipe.io(Some(deadline), None)
            .read_exact(&mut request[1..])?;
        let point = [
            i32::from_le_bytes(request[..4].try_into().unwrap()),
            i32::from_le_bytes(request[4..8].try_into().unwrap()),
        ];
        let ticks = i32::from_le_bytes(request[8..12].try_into().unwrap());
        if ticks == 0 || ticks.unsigned_abs() > 10 || request[12] > 1 {
            return Err(io::Error::other("Invalid browser scroll request"));
        }
        protocol::respond(
            &mut pipe,
            deadline,
            || provider.prepare(point, ticks, request[12] != 0),
            |prepared, deadline, attempted| provider.scroll(prepared, deadline, attempted),
        )?;
    }
}
// Only explicit transients in the query phase may pause; once Scroll is called, even a timeout cannot claim no action occurred.
pub(super) fn transient(code: i32) -> bool {
    use windows::Win32::UI::Accessibility::{UIA_E_ELEMENTNOTAVAILABLE, UIA_E_TIMEOUT};
    code == UIA_E_TIMEOUT as i32
        || code == UIA_E_ELEMENTNOTAVAILABLE as i32
        || code == 0x800705B4_u32 as i32
}
pub(super) fn failure_reply(error: &windows::core::Error, attempted: bool) -> [u8; 5] {
    let mut reply = [0; 5];
    reply[0] = if !attempted && transient(error.code().0) {
        3
    } else {
        2
    };
    reply[1..].copy_from_slice(&error.code().0.to_le_bytes());
    reply
}
