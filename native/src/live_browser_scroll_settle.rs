//! 已成功 Scroll 后有界观察 UIA 位置更新，避免下一格基于 Chromium 的旧可访问性树。
use std::time::{Duration, Instant};
use windows::core::Result;

const OBSERVATION: Duration = Duration::from_millis(750);

pub(super) fn observe(
    before: f64,
    increment: bool,
    action_deadline: Instant,
    mut percent: impl FnMut() -> Result<f64>,
) -> Result<()> {
    // -1 是 UIA 的 NoScroll；已在方向边界时允许成功的无动作，不等待不可能的位移。
    if before < 0.0 || (increment && before >= 100.0) || (!increment && before <= 0.0) {
        return Ok(());
    }
    let start = Instant::now();
    let deadline = (start + OBSERVATION).min(action_deadline - Duration::from_millis(50));
    while Instant::now() < deadline {
        let current = percent()?;
        let now = Instant::now();
        // 调用方已请求源刷新；见到新位置即可继续，不能再为每格添加固定静止等待。
        if progressed(before, current) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10).min(deadline.saturating_duration_since(now)));
    }
    // Scroll 已返回成功；CSS snap/脚本可能保持原位置。观察到期不伪造位移、重发动作，
    // 也不把合法无位移变成控制失败。RPC 错误仍传播，全部等待留在原动作预算内。
    Ok(())
}

fn progressed(before: f64, current: f64) -> bool {
    current != before
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_cache_never_counts_as_observed_progress() {
        assert!(!progressed(20.0, 20.0));
    }
    #[test]
    fn refreshed_position_does_not_add_a_quiet_delay() {
        assert!(progressed(20.0, 21.0));
        assert!(progressed(20.0, 19.0));
    }
    #[test]
    fn observation_waits_for_fresh_position_without_querying_it_again() {
        let mut calls = 0;
        observe(20.0, true, Instant::now() + Duration::from_secs(1), || {
            calls += 1;
            match calls {
                1 => Ok(20.0),
                2 => Ok(21.0),
                _ => panic!("fresh position must not start another quiet period"),
            }
        })
        .unwrap();
        assert_eq!(calls, 2);
    }
    #[test]
    fn directional_boundaries_do_not_query() {
        for (before, increment) in [(-1.0, true), (-1.0, false), (0.0, false), (100.0, true)] {
            observe(before, increment, Instant::now(), || {
                panic!("boundary query")
            })
            .unwrap();
        }
    }
    #[test]
    fn exhausted_budget_does_not_query_or_repeat_action() {
        observe(20.0, true, Instant::now(), || panic!("expired query")).unwrap();
    }
    #[test]
    fn property_failure_is_not_reported_as_success() {
        let error = observe(20.0, true, Instant::now() + Duration::from_secs(1), || {
            Err(windows::core::Error::from(
                windows::Win32::Foundation::E_ABORT,
            ))
        })
        .unwrap_err();
        assert_eq!(error.code(), windows::Win32::Foundation::E_ABORT);
    }
}
