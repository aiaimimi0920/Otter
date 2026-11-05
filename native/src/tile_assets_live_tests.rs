//! 确定性重现“检查active之后冻结、poll才返回取消”；同代际保view，换代/停止不得复活。
use super::*;
#[test]
fn freeze_after_poll_entry_holds_views_instead_of_interpreting_cancel_as_retirement() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    assert!(current(&shared, &job));
    shared.lock().frozen = true;
    assert_eq!(
        after_poll(&shared, &job, Err("Live已停止")),
        Ok(Phase::Held)
    );
    assert_eq!(after_poll(&shared, &job, Ok(())), Ok(Phase::Held));
    assert!(!current(&shared, &job));
    shared.lock().generation += 1;
    assert_eq!(
        after_poll(&shared, &job, Err("Live已停止")),
        Ok(Phase::Ended)
    );
    shared.stop();
    assert_eq!(after_poll(&shared, &job, Ok(())), Ok(Phase::Ended));
}
#[test]
fn active_network_failure_remains_a_failure_and_black_retires_even_successful_late_work() {
    let (shared, frame) = crate::tile_tests::shared_frame();
    let job = Job {
        generation: 1,
        key: frame.key,
        layout: frame.layout,
        size: frame.size,
    };
    assert_eq!(after_poll(&shared, &job, Err("broken")), Err("broken"));
    shared.lock().black = true;
    assert_eq!(after_poll(&shared, &job, Ok(())), Ok(Phase::Ended));
}
