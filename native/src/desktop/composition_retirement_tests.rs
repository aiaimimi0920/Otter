//! Retirement protocol tests use controlled completions; they do not simulate a driver fault.
use super::*;

#[test]
fn admission_is_bounded_and_does_not_execute_rejected_work() {
    static COUNT: AtomicUsize = AtomicUsize::new(MAX_WAITERS);
    let mut job = Retirement::launch(
        || panic!("rejected work ran"),
        &COUNT,
        Duration::from_secs(1),
    );
    assert_eq!(
        job.poll(),
        Status::Failed {
            stage: Stage::Admission,
            code: E_FAIL
        }
    );
    assert_eq!(COUNT.load(Ordering::Acquire), MAX_WAITERS);
}

#[test]
fn timeout_is_terminal_and_does_not_release_a_running_worker_slot() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let (release, wait) = mpsc::sync_channel(1);
    let mut job = Retirement::launch(
        move || {
            wait.recv().unwrap();
            Status::Complete
        },
        &COUNT,
        Duration::from_secs(10),
    );
    assert_eq!(job.poll(), Status::Pending);
    assert_eq!(job.poll_at(job.deadline), Status::TimedOut);
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while COUNT.load(Ordering::Acquire) != 0 {
        assert!(Instant::now() < deadline, "worker did not release its slot");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(job.poll(), Status::TimedOut);
}

#[test]
fn completion_and_original_error_are_stable_without_another_submission() {
    for expected in [
        Status::Complete,
        Status::Failed {
            stage: Stage::Commit,
            code: HRESULT(0x80070005u32 as i32),
        },
    ] {
        let (sender, receiver) = mpsc::sync_channel(1);
        let mut job = Retirement {
            receiver,
            deadline: Instant::now() + Duration::from_secs(10),
            status: Status::Pending,
        };
        sender.send(expected).unwrap();
        assert_eq!(job.poll(), expected);
        assert_eq!(job.poll_at(job.deadline), expected);
    }
}

#[test]
fn disconnected_worker_is_not_success() {
    let (sender, receiver) = mpsc::sync_channel(1);
    drop(sender);
    let mut job = Retirement {
        receiver,
        deadline: Instant::now() + Duration::from_secs(10),
        status: Status::Pending,
    };
    assert_eq!(
        job.poll(),
        Status::Failed {
            stage: Stage::Worker,
            code: E_FAIL
        }
    );
}

#[test]
fn dropping_poll_owner_does_not_join_or_cancel_the_worker() {
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    let (release, wait) = mpsc::sync_channel(1);
    let job = Retirement::launch(
        move || {
            wait.recv().unwrap();
            Status::Complete
        },
        &COUNT,
        Duration::from_secs(10),
    );
    drop(job);
    assert_eq!(COUNT.load(Ordering::Acquire), 1);
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while COUNT.load(Ordering::Acquire) != 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
