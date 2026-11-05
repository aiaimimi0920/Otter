//! Opt-in diagnostic timing for existing smoke calls; never changes acceptance deadlines.
use std::{
    ffi::OsStr,
    io::{self, Write},
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::Instant,
};

fn enabled(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}
/// 保留原等待行为；仅显式诊断时记录实际等待，不能把请求的 1 ms 当成实际耗时。
pub(in crate::desktop) fn sleep(duration: std::time::Duration) {
    let _: Result<(), std::convert::Infallible> = run("worker-pause", || {
        std::thread::sleep(duration);
        Ok(())
    });
}
pub(in crate::desktop) fn run<T, E>(
    stage: &'static str,
    action: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let requested = std::env::var_os("OTTER_NATIVE_SMOKE_TIMING");
    measure(enabled(requested.as_deref()), stage, action, |line| {
        // 诊断不可把成功操作变成 BrokenPipe panic；每行持锁写入，跨线程用 ID 配对。
        writeln!(io::stderr().lock(), "{line}")
    })
}
fn measure<T, E>(
    enabled: bool,
    stage: &'static str,
    action: impl FnOnce() -> Result<T, E>,
    mut record: impl FnMut(String) -> io::Result<()>,
) -> Result<T, E> {
    if !enabled {
        return action();
    }
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let process = std::process::id();
    let thread = std::thread::current().id();
    let _ = record(format!(
        "native-smoke-timing: stage={stage} phase=begin id={id} process={process} thread={thread:?} at_us={}",
        stamp()
    ));
    let start = Instant::now();
    let result = action();
    let _ = record(format!(
        "native-smoke-timing: stage={stage} phase=end elapsed_ms={} ok={} id={id} process={process} thread={thread:?} at_us={}",
        start.elapsed().as_millis(),
        result.is_ok(),
        stamp()
    ));
    result
}
fn stamp() -> u128 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed().as_micros()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_one_enables_diagnostics() {
        for value in [
            None,
            Some(OsStr::new("")),
            Some(OsStr::new("true")),
            Some(OsStr::new("0")),
        ] {
            assert!(!enabled(value));
        }
        assert!(enabled(Some(OsStr::new("1"))));
    }
    #[test]
    fn disabled_measurement_preserves_action_and_error_without_logging() {
        let mut calls = 0;
        let result: Result<(), &str> = measure(
            false,
            "fixture",
            || {
                calls += 1;
                Err("original")
            },
            |_| panic!("disabled log"),
        );
        assert_eq!(calls, 1);
        assert_eq!(result, Err("original"));
    }
    #[test]
    fn enabled_measurement_marks_completion_without_claiming_failed_action_passed() {
        for result in [Ok(7), Err("original")] {
            let mut lines = Vec::new();
            assert_eq!(
                measure(
                    true,
                    "fixture",
                    || result,
                    |line| {
                        lines.push(line);
                        Ok(())
                    }
                ),
                result
            );
            assert_eq!(lines.len(), 2);
            assert!(lines[0].starts_with("native-smoke-timing: stage=fixture phase=begin id="));
            assert!(
                lines[1].starts_with("native-smoke-timing: stage=fixture phase=end elapsed_ms=")
            );
            assert!(lines[1].contains(&format!("ok={} ", result.is_ok())));
            let field = |line: &str, prefix: &str| {
                line.split_ascii_whitespace()
                    .find_map(|part| part.strip_prefix(prefix))
                    .unwrap()
                    .to_string()
            };
            assert_eq!(field(&lines[0], "id="), field(&lines[1], "id="));
            assert_eq!(field(&lines[0], "process="), std::process::id().to_string());
            assert_eq!(field(&lines[0], "process="), field(&lines[1], "process="));
            assert_eq!(field(&lines[0], "thread="), field(&lines[1], "thread="));
            assert!(
                field(&lines[1], "at_us=").parse::<u128>().unwrap()
                    >= field(&lines[0], "at_us=").parse::<u128>().unwrap()
            );
        }
    }
    #[test]
    fn broken_diagnostic_pipe_preserves_success_and_original_error() {
        for result in [Ok(7), Err("original")] {
            let mut calls = 0;
            assert_eq!(
                measure(
                    true,
                    "fixture",
                    || {
                        calls += 1;
                        result
                    },
                    |_| Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
                ),
                result
            );
            assert_eq!(calls, 1);
        }
    }
    #[test]
    fn concurrent_invocations_have_distinct_pair_ids() {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(|| {
                    let mut lines = Vec::new();
                    let _: Result<(), ()> = measure(
                        true,
                        "fixture",
                        || Ok(()),
                        |line| {
                            lines.push(line);
                            Ok(())
                        },
                    );
                    let ids: Vec<_> = lines
                        .iter()
                        .map(|line| {
                            line.split_ascii_whitespace()
                                .find_map(|part| part.strip_prefix("id="))
                                .unwrap()
                                .to_string()
                        })
                        .collect();
                    assert_eq!(ids[0], ids[1]);
                    ids[0].clone()
                })
            })
            .collect();
        let ids: std::collections::HashSet<_> =
            workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(ids.len(), 4);
    }
}
