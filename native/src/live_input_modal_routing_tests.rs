//! 路由查询不消费迟到回调；完成/拒绝只在显式观察时结算，不替代真实调度回归。
use super::*;

fn assert_routed(modal: &Modal, target: &Destination, nonce: usize) {
    for _ in 0..3 {
        assert!(modal.in_flight());
        assert!(modal.escape_target().unwrap().same_target(target));
        assert_eq!(modal.token, nonce);
        assert!(modal.publication.is_some());
        assert_eq!(CALLBACK.with(Cell::get), (nonce, true));
        assert!(!modal.rejected);
    }
}

#[test]
fn late_rejection_cannot_be_consumed_by_a_route_query_after_preflight() {
    let _reset = CallbackReset::new();
    let fixture = Fixture::new();
    let (mut modal, target) = publication(&fixture, 611);
    modal.check_result().unwrap();
    // 注入前置结果检查之后的回调到达；原请求没有源 FINISHED，不能改称成功。
    unsafe { completed(target.hwnd(), 0, 611, LRESULT(0)) };
    assert_routed(&modal, &target, 611);
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 611);
    assert_eq!(protocol::value(fixture.b.0, protocol::FINISHED), 0);
    assert_eq!(modal.check_result(), Err(Error::DeliveryFailed));
    assert!(!modal.in_flight());
    assert!(modal.escape_target().is_none());
    assert_eq!(modal.check_result(), Err(Error::DeliveryFailed));
    assert_eq!(protocol::value(fixture.b.0, protocol::COMMIT), 0);
}

#[test]
fn late_success_stays_pending_until_an_explicit_result_observation() {
    let _reset = CallbackReset::new();
    let fixture = Fixture::new();
    let (mut modal, target) = publication(&fixture, 612);
    modal.check_result().unwrap();
    let wire = modal.publication.as_ref().unwrap().message().unwrap();
    unsafe { SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp)) };
    assert!(modal.publication.as_ref().unwrap().finished());
    // 成功确认由真实自有源 hook 产生；这里只模拟回调到达状态。
    unsafe { completed(target.hwnd(), wire.id, 612, LRESULT(0)) };
    assert_routed(&modal, &target, 612);
    modal.check_result().unwrap();
    assert!(!modal.in_flight());
    assert!(modal.escape_target().is_none());
    assert_eq!(modal.require_idle(), Ok(()));
}

#[test]
fn completed_callback_keeps_a_half_posted_escape_bound_to_the_original_target() {
    let _reset = CallbackReset::new();
    let fixture = Fixture::new();
    let (mut modal, target) = publication(&fixture, 613);
    modal.escape_edges = 1;
    let wire = modal.publication.as_ref().unwrap().message().unwrap();
    unsafe { SendMessageW(target.hwnd(), wire.id, Some(wire.wp), Some(wire.lp)) };
    assert!(modal.publication.as_ref().unwrap().finished());
    unsafe { completed(target.hwnd(), wire.id, 613, LRESULT(0)) };
    assert_routed(&modal, &target, 613);
    modal.check_result().unwrap();
    assert!(!modal.in_flight());
    assert!(modal.escape_target().unwrap().same_target(&target));
    assert!(modal.escape_needs_release(&target));
    assert_eq!(modal.require_idle(), Err(Error::ModalPending));
    for _ in 0..3 {
        assert!(modal.escape_target().unwrap().same_target(&target));
        assert_eq!(modal.escape_edges, 1);
    }
    assert_eq!(CALLBACK.with(Cell::get), (0, false));
}
