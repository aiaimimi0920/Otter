//! Collect separate evidence for local Art completion notification, actual exit, and slot reclamation; do not depend on post-notification scheduling speed.
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
    /// After releasing every worker gate, tests wait for actual exit rather than treating notification as terminal state.
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
                // An early-join regression waits at most five seconds, then fails explicitly on take_settled's None assertion.
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
