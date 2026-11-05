//! 私有恢复进程入口，不创建 UI、不读用户配置、不向协议流输出日志。
use crate::{live_recovery_pipe::Pipe, live_recovery_session::Session};
use std::{ffi::OsString, io, os::windows::io::AsHandle, time::Duration};

pub const FLAG: &str = "--native-recovery-parent";

/// 私有模式必须独占参数；拒绝与普通启动参数混用，不能落回 UI 启动。
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
                // 没有登记责任时可安全退出；有记录时必须继续持有 owner 等待父死。
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
            Err(_) => {
                // 仅失败恢复阶段重试；正常空闲仍完全事件驱动。失败记录不能限次后丢弃。
                std::thread::sleep(delay);
                delay = next_delay(delay);
            }
        }
    }
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
}
