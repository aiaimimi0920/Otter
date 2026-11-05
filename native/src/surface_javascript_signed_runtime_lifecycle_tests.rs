//! 有界通知和墙钟高水位竞争回归；使用原 mailbox/worker，不修改系统时钟。
use super::*;
use std::sync::Mutex;

#[test]
fn first_frame_and_final_cleanup_each_publish_before_notifying_host() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (tx, rx) = mpsc::channel();
    let (mut runtime, _grant, probe) = start_notified(
        None,
        Arc::new(move || {
            tx.send(()).unwrap();
        }),
    );
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let binding = runtime.binding.clone();
    assert!(runtime.take_reply(&binding).unwrap().is_ok());
    runtime.request_stop();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(runtime.mailbox.completed());
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
    assert!(runtime.take_exit().unwrap().is_err());
    assert!(runtime.finished());
}

#[test]
fn final_notification_observes_completion_even_before_thread_is_signaled() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (_grant, lease) = fixture::lease();
    let binding = fixture::grant(fixture::SOURCE).binding;
    let (notified, observed) = mpsc::channel();
    let (release, waiting) = mpsc::channel();
    let waiting = Mutex::new(waiting);
    // 仅测试故意暂停短通知，固定 completed 与线程句柄终态之间的窗口。
    let mut runtime = Runtime::spawn_notified(
        binding.clone(),
        lease,
        (u64::MAX, Instant::now() + Duration::from_secs(30)),
        true,
        Arc::new(move || {
            notified.send(()).unwrap();
            waiting
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
        }),
        || Err(Error::Stopped),
    )
    .unwrap();
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(runtime.mailbox.completed());
    assert!(!runtime.thread.as_ref().unwrap().is_finished());
    assert_eq!(runtime.validate(&binding), Err(Error::Stopped));
    release.send(()).unwrap();
    assert_eq!(runtime.take_exit(), Some(Err(Error::Stopped)));
    assert!(runtime.finished());
}

#[test]
fn host_and_worker_both_reject_shared_wall_clock_rollback() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    for worker_path in [false, true] {
        let (mut runtime, _grant, probe) = start(None);
        reply(&mut runtime);
        runtime.mailbox.observe(u64::MAX);
        if worker_path {
            // 绕过 UI gate 只为独立验证 worker 的重新准入，不绕过实际 worker 的检查。
            runtime.mailbox.submit(2, Operation::Suspend).unwrap();
        } else {
            let binding = runtime.binding.clone();
            assert_eq!(
                runtime.submit(&binding, Operation::Suspend),
                Err(Error::Stopped)
            );
            assert!(runtime.lease.execution_stopped());
        }
        exit(&mut runtime);
        assert!(runtime.lease.is_revoked());
        assert!(runtime.mailbox.take_reply().unwrap().is_none());
        assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
    }
}
