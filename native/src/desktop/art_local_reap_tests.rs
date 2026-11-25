//! 本地 Art 完成通知、实际退出和槽位回收分别取证；不依赖通知后的调度速度。
use super::*;
use crate::art_local_requests::tests as fixture;
use std::{
    os::windows::io::AsRawHandle,
    sync::{mpsc, Mutex},
    time::Duration,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
const WAIT: Duration = Duration::from_secs(5);

impl Tasks {
    /// 测试已释放 worker 所有门后等待真实退出，不把通知冒充终态。
    pub(crate) fn test_wait_finished(&self, index: usize) {
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(self.jobs[index].worker.as_raw_handle()), 5_000) },
            WAIT_OBJECT_0
        );
    }
}

#[test]
fn notified_art_keeps_owner_and_reaps_once_without_second_notice() {
    for panic in [false, true] {
        let (notify, noticed) = mpsc::channel();
        let (release_notice, waiting_notice) = mpsc::channel();
        let waiting_notice = Mutex::new(waiting_notice);
        let mut tasks = Tasks::new(
            "workspace:1".into(),
            Arc::new(move || {
                notify.send(()).unwrap();
                // 回归早 join 时最多等五秒，再由 take_settled 的 None 断言明确失败。
                let _ = waiting_notice.lock().unwrap().recv_timeout(WAIT);
            }),
        );
        assert!(tasks.deadline().is_none());
        let mut owner = fixture::owner();
        let mut authority = fixture::authority();
        let lease = fixture::lease(&mut owner);
        let (release_work, waiting_work) = mpsc::channel();
        tasks
            .start_with(
                fixture::id(),
                fixture::request(),
                lease.clone(),
                &mut authority,
                move |_| {
                    waiting_work.recv_timeout(WAIT).unwrap();
                    assert!(!panic, "injected local Art worker panic");
                    Err(Error::Worker)
                },
            )
            .unwrap();
        assert!(tasks.deadline().is_none());
        assert!(tasks.take_settled().is_none());
        release_work.send(()).unwrap();
        noticed.recv_timeout(WAIT).unwrap();
        assert!(tasks.deadline().is_some());
        assert!(tasks.take_settled().is_none());
        assert!(tasks.busy());
        assert_eq!(tasks.jobs.len(), 1);
        assert!(!lease.is_revoked());
        release_notice.send(()).unwrap();
        tasks.test_wait_finished(0);
        assert!(matches!(
            tasks.take_settled().unwrap().result,
            Err(Error::Worker)
        ));
        assert!(lease.is_revoked());
        assert!(!tasks.busy());
        assert!(tasks.deadline().is_none());
        assert!(tasks.take_settled().is_none());
        assert!(noticed.try_recv().is_err());
    }
}
