//! 真实 overlapped Pipe 的分阶段回归；假 provider 不访问桌面、不发送输入。
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};

fn peer(
    prepare: impl FnOnce() -> windows::core::Result<Option<()>> + Send + 'static,
    apply: impl FnOnce((), Instant, &mut bool) -> windows::core::Result<bool> + Send + 'static,
) -> (Pipe, JoinHandle<io::Result<()>>) {
    let (parent, mut child) = Pipe::pair().unwrap();
    let task = thread::spawn(move || {
        let mut request = [0; 13];
        child.io(Some(after(1000)), None).read_exact(&mut request)?;
        respond(&mut child, after(QUERY_TIMEOUT_MS - 50), prepare, apply)
    });
    (parent, task)
}

#[test]
fn prepared_commit_can_arrive_after_query_deadline() {
    let (mut parent, mut child) = Pipe::pair().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let query_deadline = after(500);
    let task = thread::spawn(move || {
        respond(
            &mut child,
            query_deadline,
            || Ok(Some(())),
            move |_, _, attempted| {
                *attempted = true;
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(true)
            },
        )
    });
    let mut channel = parent.io(Some(after(2000)), None);
    let mut reply = [0; 5];
    channel.read_exact(&mut reply).unwrap();
    assert_eq!(reply, [PREPARED, 0, 0, 0, 0]);
    thread::sleep(
        query_deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(100),
    );
    channel.write_all(&[COMMIT]).unwrap();
    channel.read_exact(&mut reply).unwrap();
    assert_eq!(reply, [1, 0, 0, 0, 0]);
    task.join().unwrap().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn peer_exit_after_prepared_is_not_a_query_transition() {
    let (mut parent, mut child) = Pipe::pair().unwrap();
    let task = thread::spawn(move || {
        let mut channel = child.io(Some(after(1000)), None);
        channel.read_exact(&mut [0; 13]).unwrap();
        channel.write_all(&[PREPARED, 0, 0, 0, 0]).unwrap();
    });
    let error = request(&mut parent, None, &[0; 13]).unwrap_err();
    task.join().unwrap();
    assert_ne!(error.kind(), io::ErrorKind::WouldBlock);
}

#[test]
fn successful_two_second_action_keeps_its_acknowledgement() {
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let (mut pipe, task) = peer(
        || Ok(Some(())),
        move |_, _, attempted| {
            *attempted = true;
            observed.fetch_add(1, Ordering::SeqCst);
            // 旧 700/750ms 协议会丢掉已成功的结果；复现实测两秒 UIA 返回。
            thread::sleep(Duration::from_millis(2100));
            Ok(true)
        },
    );
    let result = request(&mut pipe, None, &[0; 13]);
    drop(pipe);
    task.join().unwrap().unwrap();
    assert_eq!(result.unwrap(), [1, 0, 0, 0, 0]);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn query_timeout_never_grants_an_action_even_if_query_later_completes() {
    let (release, ready) = mpsc::channel();
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let (mut pipe, task) = peer(
        move || {
            ready.recv_timeout(Duration::from_secs(2)).unwrap();
            Ok(Some(()))
        },
        move |_, _, _| {
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        },
    );
    let result = request_with_budgets(&mut pipe, None, &[0; 13], 120, 500);
    drop(pipe);
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[test]
fn action_timeout_is_unknown_and_late_success_is_not_replayed() {
    let (release, ready) = mpsc::channel();
    let count = Arc::new(AtomicUsize::new(0));
    let observed = count.clone();
    let (mut pipe, task) = peer(
        || Ok(Some(())),
        move |_, _, attempted| {
            *attempted = true;
            observed.fetch_add(1, Ordering::SeqCst);
            ready.recv_timeout(Duration::from_secs(2)).unwrap();
            Ok(true)
        },
    );
    let result = request_with_budgets(&mut pipe, None, &[0; 13], 750, 120);
    drop(pipe);
    release.send(()).unwrap();
    assert!(task.join().unwrap().is_err());
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn prepared_query_without_parent_commit_cannot_run() {
    for commit in [None, Some(9)] {
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let (mut pipe, task) = peer(
            || Ok(Some(())),
            move |_, _, _| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(true)
            },
        );
        let mut channel = pipe.io(Some(after(750)), None);
        channel.write_all(&[0; 13]).unwrap();
        let mut reply = [0; 5];
        channel.read_exact(&mut reply).unwrap();
        assert_eq!(reply, [PREPARED, 0, 0, 0, 0]);
        if let Some(value) = commit {
            channel.write_all(&[value]).unwrap();
        }
        drop(pipe);
        assert!(task.join().unwrap().is_err());
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn unsupported_target_does_not_enter_action_phase() {
    let (mut pipe, task) = peer(|| Ok(None), |_, _, _| panic!("no scroll target"));
    assert_eq!(request(&mut pipe, None, &[0; 13]).unwrap(), [0; 5]);
    task.join().unwrap().unwrap();
}

#[test]
fn invalid_prepared_reply_is_rejected_without_commit() {
    for reply in [[1, 0, 0, 0, 0], [PREPARED, 1, 0, 0, 0], [9, 0, 0, 0, 0]] {
        let (mut parent, mut child) = Pipe::pair().unwrap();
        let task = thread::spawn(move || {
            let mut channel = child.io(Some(after(1000)), None);
            channel.read_exact(&mut [0; 13]).unwrap();
            channel.write_all(&reply).unwrap();
            channel.read_exact(&mut [0])
        });
        let result = request(&mut parent, None, &[0; 13]);
        drop(parent);
        assert!(task.join().unwrap().is_err());
        assert!(result.is_err());
    }
}
