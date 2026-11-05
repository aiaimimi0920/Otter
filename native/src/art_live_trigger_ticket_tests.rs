//! 固定墙钟/单调期限及Stop与canonical事务的串行围栏，不修改系统时钟。
use super::*;
use crate::art_live_trigger::tests as f;
#[test]
fn fixed_deadline_wall_high_water_and_binding_are_not_extended_by_queries() {
    let (_, t) = f::ticket(f::binding());
    let now = t.0.observed_wall.load(Ordering::Acquire);
    assert!(t.time_current_at(now, t.0.deadline - Duration::from_nanos(1)));
    assert!(!t.time_current_at(now, t.0.deadline));
    assert!(!t.time_current_at(t.event().delivery.expires_at_ms, Instant::now()));
    assert!(!t.time_current_at(now, Instant::now())); // 前跳后不能回退为有效。
    let (gate, t) = f::ticket(f::binding());
    let mut other = f::binding();
    other.generation += 1;
    assert!(!t.current(&other));
    assert!(!gate.active());
    assert!(!t.current(t.binding()));
}
#[test]
fn stop_and_actual_transaction_are_serialized_and_late_commit_is_rejected() {
    let (gate, t) = f::ticket(f::binding());
    let marker = std::sync::Arc::new(AtomicU64::new(0));
    let (entered, inside) = std::sync::mpsc::sync_channel(0);
    let (release, waiting) = std::sync::mpsc::sync_channel(0);
    std::thread::scope(|s| {
        let t2 = t.clone();
        let m = marker.clone();
        let apply = s.spawn(move || {
            t2.commit(t2.binding(), |allowed| {
                entered.send(()).unwrap();
                waiting.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(allowed());
                m.store(1, Ordering::Release);
            })
        });
        inside.recv_timeout(Duration::from_secs(5)).unwrap();
        let m = marker.clone();
        let cancel = s.spawn(move || {
            gate.stop();
            assert_eq!(m.load(Ordering::Acquire), 1);
        });
        release.send(()).unwrap();
        assert!(apply.join().unwrap().is_some());
        cancel.join().unwrap();
    });
    assert!(t
        .commit(t.binding(), |_| panic!("cancelled commit"))
        .is_none());
    assert!(!t.complete(Receipt::Applied { formal_revision: 1 }));
}
