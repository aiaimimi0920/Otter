//! 模拟后台提供方，不访问用户剪贴板；通道握手而非依赖线程调度时序。
use super::*;
use crate::clipboard_history::Status;
use std::time::{Duration, Instant};

fn result(task: &mut Task) -> Result<Snapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = task.poll() {
            return value;
        }
        assert!(Instant::now() < deadline, "Worker did not finish");
        std::thread::yield_now();
    }
}

#[test]
fn result_is_delivered_once_and_worker_failure_is_reported() {
    let mut task = Task::spawn(|_, _permit| {
        Ok(Snapshot {
            status: Status::Ready,
            entries: Vec::new(),
        })
    })
    .unwrap();
    assert_eq!(result(&mut task).unwrap().status, Status::Ready);
    assert!(task.poll().is_none());
    let mut failed = Task::spawn(|_, _permit| panic!("Synthetic provider failure")).unwrap();
    assert_eq!(result(&mut failed).err().unwrap().code(), E_FAIL);
    assert!(failed.poll().is_none());
}

#[test]
fn dismissal_detaches_but_keeps_permit_until_provider_returns() {
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (finished_tx, finished_rx) = mpsc::sync_channel(1);
    let task = Task::spawn(move |cancel, permit| {
        started_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(cancel.load(Ordering::Acquire));
        drop(permit);
        finished_tx.send(()).unwrap();
        Ok(Snapshot {
            status: Status::Ready,
            entries: Vec::new(),
        })
    })
    .unwrap();
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(task); // 若 Drop 等待，提供方将超时并导致以下许可断言失败。
    assert!(Task::spawn(|_, _| panic!("Busy worker must not spawn")).is_err());
    release_tx.send(()).unwrap();
    finished_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut next = Task::spawn(|_, _| {
        Ok(Snapshot {
            status: Status::Disabled,
            entries: Vec::new(),
        })
    })
    .unwrap();
    assert_eq!(result(&mut next).unwrap().status, Status::Disabled);
}

#[test]
fn explicit_cancel_discards_already_queued_result() {
    let cancel = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::sync_channel(1);
    sender
        .send(Ok(Snapshot {
            status: Status::Ready,
            entries: Vec::new(),
        }))
        .unwrap();
    let mut task = Task {
        cancel: Arc::clone(&cancel),
        receiver: Some(receiver),
    };
    task.cancel();
    task.cancel();
    assert!(cancel.load(Ordering::Acquire));
    assert!(task.poll().is_none());
    assert!(sender
        .send(Ok(Snapshot {
            status: Status::Ready,
            entries: Vec::new(),
        }))
        .is_err());
}
