//! 单在途桥只确认真实执行回复；停止越过等待队列，不依赖UI再次消费请求。
use super::*;
use std::time::{Duration, Instant};
fn take(bridge: &Bridge) -> Request {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if let Some(request) = bridge.take() {
            return request;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn source_input_bridge_waits_for_actual_reply_and_rejects_second_inflight() {
    let bridge = Bridge::new();
    let worker = bridge.clone();
    let thread = std::thread::spawn(move || worker.dispatch(Action::Enable, || true));
    let request = take(&bridge);
    assert!(!thread.is_finished());
    assert_eq!(bridge.dispatch(Action::Release, || true), Err(Error::Busy));
    request.finish(Ok(()));
    assert_eq!(thread.join().unwrap(), Ok(()));
    bridge.cancel();
    assert_eq!(
        bridge.dispatch(Action::Enable, || true),
        Err(Error::Stopped)
    );
}
#[test]
fn source_input_bridge_cancel_discards_late_request_without_ack_or_requeue() {
    let bridge = Bridge::new();
    let worker = bridge.clone();
    let thread = std::thread::spawn(move || {
        worker.dispatch(
            Action::Input(Event::Key {
                code: 65,
                down: true,
            }),
            || true,
        )
    });
    let request = take(&bridge);
    bridge.cancel();
    assert_eq!(thread.join().unwrap(), Err(Error::Stopped));
    request.finish(Ok(()));
    assert!(bridge.take().is_none());
}
