//! Completion notification is not thread exit: the UI must not join while notification is blocked; cancellation/failure still deliver only once.
use super::*;
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

impl Job {
    /// Completion may arrive first; TLS and lifecycle regressions wait for actual thread exit before collecting.
    pub(crate) fn test_wait_finished(&self) {
        let worker = &self.running.as_ref().unwrap().worker;
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(worker.as_raw_handle()), 5_000) },
            WAIT_OBJECT_0
        );
    }
}

#[test]
fn notified_catalog_reaps_once_without_waiting_for_callback_or_second_notice() {
    for cancel in [false, true] {
        let (notify, noticed) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let waiting = Mutex::new(waiting);
        let mut job = Job::default();
        assert!(job.deadline().is_none());
        // Use production startup/completion paths; disable discovery and avoid development-machine manifests or networking.
        let config = loom_discovery::Config::from_lookup(|key| {
            (key == "OTTER_ENABLE_LOOM_OTTER").then(|| "false".into())
        });
        job.start(
            Source::Discovery(config),
            Arc::new(move || {
                notify.send(()).unwrap();
                // Early-join regressions must fail within bounds rather than hang the test itself.
                let _ = waiting.lock().unwrap().recv_timeout(WAIT);
            }),
        )
        .unwrap();
        noticed.recv_timeout(WAIT).unwrap();
        assert!(job.deadline().is_some());
        assert!(job.poll().is_none());
        assert!(job.is_busy());
        if cancel {
            job.cancel();
        }
        release.send(()).unwrap();
        job.test_wait_finished();
        let result = job.poll().unwrap();
        if cancel {
            assert!(matches!(result, Err(Error::Cancelled)));
        } else {
            assert!(matches!(
                result,
                Err(Error::Discovery(loom_discovery::Error::Disabled))
            ));
        }
        assert!(!job.is_busy());
        assert!(job.deadline().is_none());
        assert!(job.poll().is_none());
        assert!(noticed.try_recv().is_err());
    }
}

#[test]
fn unfinished_catalog_has_no_reap_deadline_and_panic_remains_worker_failure() {
    let (release, waiting) = mpsc::channel();
    let (notify, noticed) = mpsc::channel();
    let ready = Arc::new(AtomicBool::new(false));
    let completed = ready.clone();
    let worker = thread::spawn(move || {
        let _signal = CompletionSignal::new(
            completed,
            Arc::new(move || {
                notify.send(()).unwrap();
            }),
        );
        waiting.recv_timeout(WAIT).unwrap();
        panic!("injected catalog worker panic");
    });
    let mut job = Job {
        running: Some(Running {
            cancel: Arc::new(AtomicBool::new(false)),
            ready,
            worker,
        }),
    };
    assert!(job.poll().is_none());
    assert!(job.deadline().is_none());
    release.send(()).unwrap();
    noticed.recv_timeout(WAIT).unwrap();
    job.test_wait_finished();
    assert!(matches!(job.poll(), Some(Err(Error::WorkerFailed))));
    assert!(job.deadline().is_none());
}
