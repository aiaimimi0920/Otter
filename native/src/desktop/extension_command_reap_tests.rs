//! Completion signals are not thread exit; success, failure, panic, and cancellation retain owners until actual exit.
use super::*;
use crate::desktop::loom_execution_tasks::reap_fixture::{Gate, WAIT};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn completion_tail_keeps_job_until_exit_and_preserves_each_result_once() {
    for outcome in 0..4 {
        let gate = Gate::new(|| {});
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let notify = gate.notify.clone();
        let (start, waiting) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _signal = CompletionSignal::new(completed, notify);
            waiting.recv_timeout(WAIT).unwrap();
            match outcome {
                0 => Ok(()),
                1 => Err("original extension error".into()),
                2 => panic!("injected extension worker panic"),
                _ => Err("extension command: Cancelled".into()),
            }
        });
        let mut job = Job {
            port: Port {
                mailbox: Arc::new(Mutex::new(Mailbox::default())),
                cancel: Arc::new(AtomicBool::new(false)),
                notify: Arc::new(|| {}),
            },
            ready,
            worker: Some(worker),
        };
        assert!(job.deadline().is_none() && job.poll().is_none());
        if outcome >= 2 {
            job.cancel();
        }
        start.send(()).unwrap();
        gate.noticed.recv_timeout(WAIT).unwrap();
        let before = Instant::now();
        assert!(job.poll().is_none());
        assert!(before.elapsed() < Duration::from_secs(1));
        assert!(job.worker.is_some() && job.deadline().is_some());
        gate.release.send(()).unwrap();
        job.test_wait_finished();
        assert_eq!(
            job.poll().unwrap(),
            match outcome {
                0 => Ok(()),
                1 => Err("original extension error".into()),
                2 => Err("extension worker failed".into()),
                _ => Err("extension command: Cancelled".into()),
            }
        );
        assert!(job.poll().is_none() && job.deadline().is_none());
        assert!(gate.noticed.try_recv().is_err());
    }
}

#[test]
fn panicking_request_notification_discards_packet_before_completion_notification() {
    let gate = Gate::new(|| {});
    let ready = Arc::new(AtomicBool::new(false));
    let completed = ready.clone();
    let notify = gate.notify.clone();
    let port = Port {
        mailbox: Arc::new(Mutex::new(Mailbox::default())),
        cancel: Arc::new(AtomicBool::new(false)),
        notify: Arc::new(|| panic!("injected request notification panic")),
    };
    let worker_port = port.clone();
    let worker = thread::spawn(move || {
        let _signal = CompletionSignal::new(completed, notify);
        let _published = Published(worker_port.mailbox.clone());
        worker_port
            .ask(super::super::extension_command_bridge::Request::Toggle { translation: false })
            .map(|_| ())
            .map_err(diagnostic)
    });
    let mut job = Job {
        port,
        ready,
        worker: Some(worker),
    };
    gate.noticed.recv_timeout(WAIT).unwrap();
    assert!(!job.request_pending());
    assert!(job.take_request().is_none());
    assert!(!job.port.cancel.load(std::sync::atomic::Ordering::Acquire));
    assert!(job.poll().is_none());
    gate.release.send(()).unwrap();
    job.test_wait_finished();
    assert_eq!(job.poll(), Some(Err("extension worker failed".into())));
    assert!(job.deadline().is_none());
}
