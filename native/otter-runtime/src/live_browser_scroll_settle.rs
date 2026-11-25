//! Bounded observation of UIA position updates after successful Scroll prevents the next notch from using Chromium's stale accessibility tree.
use std::time::{Duration, Instant};
use windows::core::Result;

const OBSERVATION: Duration = Duration::from_millis(750);

pub(super) fn observe(
    before: f64,
    increment: bool,
    action_deadline: Instant,
    mut percent: impl FnMut() -> Result<f64>,
) -> Result<()> {
    // -1 is UIA NoScroll; allow a successful no-op at the directional boundary without waiting for impossible movement.
    if before < 0.0 || (increment && before >= 100.0) || (!increment && before <= 0.0) {
        return Ok(());
    }
    let start = Instant::now();
    let deadline = (start + OBSERVATION).min(action_deadline - Duration::from_millis(50));
    while Instant::now() < deadline {
        let current = percent()?;
        let now = Instant::now();
        // The caller already requested a source refresh; continue once a new position appears, without adding fixed idle waits per notch.
        if progressed(before, current) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10).min(deadline.saturating_duration_since(now)));
    }
    // Scroll already returned success; CSS snap/scripts may preserve the position. Observation expiry must not fake movement, replay actions,
    // or turn legitimate nonmovement into control failure. RPC errors still propagate, and all waiting stays within the original action budget.
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
