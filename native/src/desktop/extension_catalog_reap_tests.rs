//! 真实订阅在完成通知尾段先撤权，线程退出后才回收；中途快照通知不启用回收期限。
use super::*;
use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};

#[test]
fn completion_tail_revokes_once_without_join_and_reaps_closed_or_cancelled_result() {
    let _serial = SERIAL.lock().unwrap();
    for cancelled in [false, true] {
        let (advance, waiting) = mpsc::channel();
        let (identity, server) = peer::peer(move |p| {
            peer::handshake(p, peer::handshake_data());
            waiting.recv_timeout(WAIT).unwrap();
            if cancelled {
                assert!(p.read().is_err());
            } else {
                p.close(None).unwrap();
            }
        });
        let gate = Gate::new(|| {});
        let tail = gate.notify.clone();
        let (snapshot_sent, snapshot_notice) = mpsc::channel();
        let count = AtomicUsize::new(0);
        let notify = Arc::new(move || {
            if count.fetch_add(1, Ordering::AcqRel) == 0 {
                snapshot_sent.send(()).unwrap();
            } else {
                tail();
            }
        });
        let mut job = Job::default();
        assert!(job.deadline().is_none());
        job.start(Source::Fixed(identity.clone()), notify).unwrap();
        snapshot_notice.recv_timeout(WAIT).unwrap();
        assert!(job.deadline().is_none());
        // 故意不领取初始快照；终态不得复活它。
        if cancelled {
            job.cancel();
        }
        advance.send(()).unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        assert!(
            !ACTIVE.load(Ordering::Acquire),
            "network permit must already be released"
        );
        let start = Instant::now();
        assert!(matches!(job.poll(), Some(Event::Revoked)));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(job.is_busy() && job.poll().is_none());
        assert!(job.deadline().is_some());
        assert!(matches!(
            job.start(Source::Fixed(identity), Arc::new(|| {})),
            Err(Error::Busy)
        ));
        gate.release.send(()).unwrap();
        job.test_wait_finished();
        match job.poll() {
            Some(Event::Stopped(Err(Error::Cancelled))) => assert!(cancelled),
            Some(Event::Stopped(Err(Error::Exchange(ExchangeError::Closed)))) => {
                assert!(!cancelled)
            }
            _ => panic!("unexpected catalog completion"),
        }
        assert!(!job.is_busy() && job.deadline().is_none() && job.poll().is_none());
        assert!(
            gate.noticed.try_recv().is_err(),
            "must not need a second completion notification"
        );
        server.join().unwrap();
    }
}
