//! Private recovery-process entry point; no UI creation, user-configuration reads, or logging to the protocol stream.
use crate::{live_recovery_pipe::Pipe, live_recovery_session::Session};
use std::{ffi::OsString, io, os::windows::io::AsHandle, time::Duration};

pub const FLAG: &str = "--native-recovery-parent";

/// Private mode must exclusively own arguments; reject mixing with ordinary startup arguments and never fall back to UI startup.
pub fn parse(args: &[OsString]) -> io::Result<Option<u32>> {
    if !args.iter().any(|arg| arg == FLAG) {
        return Ok(None);
    }
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid native recovery invocation",
        )
    };
    if args.len() != 2 || args[0] != FLAG {
        return Err(invalid());
    }
    let text = args[1].to_str().ok_or_else(invalid)?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let pid = text.parse::<u32>().map_err(|_| invalid())?;
    if pid == 0 {
        return Err(invalid());
    }
    Ok(Some(pid))
}

pub fn run_inherited(parent: u32) -> io::Result<()> {
    let input = std::io::stdin();
    let pipe = Pipe::adopt(input.as_handle().try_clone_to_owned()?)?;
    run(Session::accept(parent, pipe)?)
}

fn run(mut session: Session) -> io::Result<()> {
    loop {
        match session.step() {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(error) => {
                // Exit safely without registration responsibility; with records, retain the owner and wait for parent death.
                if session.pending() == 0 {
                    return Err(error);
                }
                break;
            }
        }
    }
    let mut delay = Duration::from_secs(1);
    loop {
        match session.wait_and_recover() {
            Ok(()) => return Ok(()),
            Err(error) => {
                if error.kind() != io::ErrorKind::WouldBlock {
                    eprintln!("native-live-recovery-restore: {error}");
                }
                // Pending keeps observing the original single slot without resending; normal asynchronous startup must not increase failure backoff.
                std::thread::sleep(recovery_delay(&error, &mut delay));
            }
        }
    }
}
fn recovery_delay(error: &io::Error, delay: &mut Duration) -> Duration {
    if error.kind() == io::ErrorKind::WouldBlock {
        return crate::live_popup_layer_request::POLL;
    }
    // Retain the original bounded backoff only for real failures; failed registrations must not be discarded after a retry limit.
    let current = *delay;
    *delay = next_delay(current);
    current
}
fn next_delay(delay: Duration) -> Duration {
    delay.saturating_mul(2).min(Duration::from_secs(30))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_mode_is_exclusive_and_pid_is_strict() {
        let args = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(parse(&args(&[FLAG, "123"])).unwrap(), Some(123));
        assert_eq!(parse(&args(&["--capture"])).unwrap(), None);
        for values in [
            vec![FLAG],
            vec![FLAG, "0"],
            vec![FLAG, "-1"],
            vec![FLAG, "+1"],
            vec![FLAG, "4294967296"],
            vec![FLAG, "1", "--settings", "x"],
            vec!["--capture", FLAG, "1"],
        ] {
            assert!(parse(&args(&values)).is_err());
        }
    }
    #[test]
    fn recovery_backoff_is_capped_without_overflow() {
        let mut delay = Duration::from_secs(1);
        for expected in [2, 4, 8, 16, 30, 30] {
            delay = next_delay(delay);
            assert_eq!(delay, Duration::from_secs(expected));
        }
        assert_eq!(next_delay(Duration::MAX), Duration::from_secs(30));
    }
    #[test]
    fn source_pending_does_not_increase_or_reset_actual_failure_backoff() {
        let pending = io::Error::from(io::ErrorKind::WouldBlock);
        let failed = io::Error::other("actual source failure");
        let mut delay = Duration::from_secs(1);
        for _ in 0..3 {
            assert_eq!(
                recovery_delay(&pending, &mut delay),
                crate::live_popup_layer_request::POLL
            );
            assert_eq!(delay, Duration::from_secs(1));
        }
        assert_eq!(recovery_delay(&failed, &mut delay), Duration::from_secs(1));
        assert_eq!(delay, Duration::from_secs(2));
        assert_eq!(
            recovery_delay(&pending, &mut delay),
            crate::live_popup_layer_request::POLL
        );
        assert_eq!(delay, Duration::from_secs(2));
        assert_eq!(recovery_delay(&failed, &mut delay), Duration::from_secs(2));
        assert_eq!(delay, Duration::from_secs(4));
    }
}
