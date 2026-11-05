//! 显式文件事务计时：不记录路径、内容或错误文本，不参与准入/取消/持久化策略。
use std::{
    ffi::OsStr,
    io::{self, Write},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

fn enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

pub(crate) fn run<T, E>(
    stage: &'static str,
    bytes: Option<usize>,
    action: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let requested = std::env::var_os("OTTER_NATIVE_SMOKE_TIMING");
    measure(
        enabled(requested.as_deref()),
        stage,
        bytes,
        action,
        |line| {
            // 诊断管道断开不能把成功的文件事务变成失败或 panic。
            writeln!(io::stderr().lock(), "{line}")
        },
    )
}

fn measure<T, E>(
    enabled: bool,
    stage: &'static str,
    bytes: Option<usize>,
    action: impl FnOnce() -> Result<T, E>,
    mut record: impl FnMut(String) -> io::Result<()>,
) -> Result<T, E> {
    if !enabled {
        return action();
    }
    // ID 只供同一进程内并发日志配对，不是文件身份、权限或业务凭证。
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let bytes = bytes.map_or_else(|| "unknown".into(), |value| value.to_string());
    let _ = record(format!(
        "native-file-timing: id={id} stage={stage} phase=begin bytes={bytes}"
    ));
    let start = Instant::now();
    let result = action();
    let _ = record(format!(
        "native-file-timing: id={id} stage={stage} phase=end elapsed_us={} ok={}",
        start.elapsed().as_micros(),
        result.is_ok()
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_one_enables_file_diagnostics() {
        for value in [None, Some(OsStr::new("true")), Some(OsStr::new("0"))] {
            assert!(!enabled(value));
        }
        assert!(enabled(Some(OsStr::new("1"))));
    }
    #[test]
    fn disabled_trace_runs_once_and_preserves_original_error() {
        let mut calls = 0;
        let result: Result<(), &str> = measure(
            false,
            "fixture",
            Some(7),
            || {
                calls += 1;
                Err("original")
            },
            |_| panic!("disabled diagnostics must not log"),
        );
        assert_eq!(calls, 1);
        assert_eq!(result, Err("original"));
    }
    #[test]
    fn records_pair_and_status_without_error_details() {
        for result in [Ok(7), Err("secret-path-or-content")] {
            let mut lines = Vec::new();
            assert_eq!(
                measure(
                    true,
                    "fixture",
                    Some(11),
                    || result,
                    |line| {
                        lines.push(line);
                        Ok(())
                    }
                ),
                result
            );
            assert_eq!(lines.len(), 2);
            assert!(lines[0].ends_with("stage=fixture phase=begin bytes=11"));
            assert!(lines[1].contains("stage=fixture phase=end elapsed_us="));
            assert!(lines[1].ends_with(&format!("ok={}", result.is_ok())));
            let id = |line: &str| line.split_ascii_whitespace().nth(1).unwrap().to_owned();
            assert_eq!(id(&lines[0]), id(&lines[1]));
            assert!(lines
                .iter()
                .all(|line| !line.contains("secret-path-or-content")));
        }
    }
    #[test]
    fn broken_diagnostic_output_does_not_change_transaction_result() {
        for result in [Ok(7), Err("original")] {
            assert_eq!(
                measure(
                    true,
                    "fixture",
                    None,
                    || result,
                    |_| { Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")) }
                ),
                result
            );
        }
    }
}
