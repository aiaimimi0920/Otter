//! 异步终态不需要新输入，也不能充当其他命令的 ACK；闸门有界且先于 Worker 释放。
use super::*;
use crate::live_input::Delivery;
use std::sync::atomic::AtomicUsize;

#[derive(Default)]
struct State {
    pending: AtomicBool,
    reject: AtomicBool,
    finish_in_pending: AtomicBool,
    preflights: AtomicUsize,
}
struct Probe {
    inner: Mock,
    state: Arc<State>,
    gate: Option<(mpsc::SyncSender<()>, mpsc::Receiver<()>)>,
    arm_at: usize,
    rejected: bool,
}
impl Backend for Probe {
    type Target = u8;
    type Error = &'static str;
    fn idle(&mut self) -> Result<(), Self::Error> {
        if self.state.pending.load(Ordering::Acquire) {
            if let Some((entered, resume)) = self.gate.take() {
                entered.send(()).map_err(|_| "idle gate disconnected")?;
                resume
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| "idle gate timed out")?;
            }
            if self.state.reject.load(Ordering::Acquire) {
                self.rejected = true;
            }
        }
        if self.rejected {
            Err("asynchronous delivery rejected")
        } else {
            Ok(())
        }
    }
    fn pending_delivery(&mut self) -> bool {
        // 模拟生产 Modal::pending 在查询内处理完成回调并清 token。
        if self.state.finish_in_pending.swap(false, Ordering::AcqRel) {
            self.rejected = true;
            self.state.pending.store(false, Ordering::Release);
        }
        self.state.pending.load(Ordering::Acquire)
    }
    fn delivery(&mut self) -> Delivery {
        if self.state.pending.load(Ordering::Acquire) {
            Delivery::Pending
        } else {
            Delivery::Completed
        }
    }
    fn preflight(&mut self) -> Result<(), Self::Error> {
        let preflights = self.state.preflights.fetch_add(1, Ordering::AcqRel) + 1;
        if self.arm_at != 0 && preflights == self.arm_at {
            self.state.pending.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn resolve(&mut self, event: Event, preferred: Option<u8>) -> Result<u8, Self::Error> {
        self.inner.resolve(event, preferred)
    }
    fn send(
        &mut self,
        target: u8,
        event: Event,
        buttons: u8,
        timeout: u32,
    ) -> Result<(), Self::Error> {
        self.inner.send(target, event, buttons, timeout)?;
        self.state.pending.store(true, Ordering::Release);
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        self.inner.cleanup_target(target)
    }
    fn release(&mut self, target: u8, event: Event, buttons: u8) -> Result<(), Self::Error> {
        self.inner.release(target, event, buttons)
    }
    fn leave(&mut self) -> Cleanup {
        self.state.pending.store(false, Ordering::Release);
        Cleanup::default()
    }
}
struct Resume(Option<mpsc::SyncSender<()>>);
impl Resume {
    fn open(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.try_send(());
        }
    }
}
impl Drop for Resume {
    fn drop(&mut self) {
        self.open();
    }
}
struct Harness {
    // 字段析构顺序保证断言失败也会先打开闸门，再等待 Worker。
    resume: Resume,
    worker: Worker<&'static str>,
    entered: mpsc::Receiver<()>,
    state: Arc<State>,
    trace: Arc<Mutex<Trace>>,
}
impl Harness {
    fn new(arm_at: usize) -> Self {
        let state = Arc::new(State::default());
        let trace = Arc::new(Mutex::new(Trace::default()));
        let (entered_tx, entered) = mpsc::sync_channel(1);
        let (resume_tx, resume_rx) = mpsc::sync_channel(1);
        let backend = Probe {
            inner: Mock(trace.clone()),
            state: state.clone(),
            gate: Some((entered_tx, resume_rx)),
            arm_at,
            rejected: false,
        };
        Self {
            resume: Resume(Some(resume_tx)),
            worker: Worker::start(move || Ok(backend)).unwrap(),
            entered,
            state,
            trace,
        }
    }
    fn ready(&self) {
        self.entered.recv_timeout(Duration::from_secs(5)).unwrap();
    }
    fn pending_key(&mut self) {
        self.worker.submit(Command::Enable).unwrap();
        wait_reply(&mut self.worker).result.unwrap();
        self.worker
            .submit(Command::Input {
                sequence: 1,
                event: Event::Key {
                    code: 65,
                    down: true,
                },
            })
            .unwrap();
        assert_eq!(wait_reply(&mut self.worker).result, Ok(Delivery::Pending));
        self.ready();
    }
    fn reject(&mut self) {
        self.state.reject.store(true, Ordering::Release);
        self.resume.open();
    }
    fn terminal(&mut self) {
        let reply = wait_reply(&mut self.worker);
        assert!(reply.command.is_none());
        assert_eq!(
            reply.result,
            Err(Error::Backend("asynchronous delivery rejected"))
        );
    }
}

#[test]
fn asynchronous_failure_reports_and_releases_without_another_command() {
    let mut fixture = Harness::new(0);
    fixture.pending_key();
    fixture.reject();
    fixture.terminal();
    let exit = finish(&mut fixture.worker);
    assert_eq!(exit.reason, Reason::InputFailed);
    assert_eq!(
        exit.cleanup,
        Some(Cleanup {
            released: 1,
            ..Default::default()
        })
    );
    assert_eq!(fixture.trace.lock().unwrap().released, 1);
    assert_eq!(fixture.trace.lock().unwrap().sent.len(), 1);
}

#[test]
fn terminal_event_does_not_ack_an_unexecuted_command() {
    let mut fixture = Harness::new(0);
    fixture.pending_key();
    fixture.worker.submit(Command::Enable).unwrap();
    assert_eq!(fixture.worker.outstanding(), 1);
    fixture.reject();
    fixture.terminal();
    assert_eq!(fixture.worker.outstanding(), 1);
    assert_eq!(fixture.state.preflights.load(Ordering::Acquire), 2);
    assert_eq!(finish(&mut fixture.worker).reason, Reason::InputFailed);
    assert_eq!(fixture.trace.lock().unwrap().released, 1);
}

#[test]
fn terminal_slot_survives_sixteen_unconsumed_command_replies() {
    let mut fixture = Harness::new(CAPACITY);
    for _ in 0..CAPACITY {
        fixture.worker.submit(Command::Enable).unwrap();
    }
    fixture.ready();
    assert_eq!(fixture.worker.outstanding(), CAPACITY);
    fixture.reject();
    let done = unsafe {
        WaitForSingleObject(
            HANDLE(fixture.worker.completion_handle().unwrap().as_raw_handle()),
            5000,
        )
    };
    assert_eq!(done, WAIT_OBJECT_0, "terminal publish must not block");
    for _ in 0..CAPACITY {
        let reply = wait_reply(&mut fixture.worker);
        assert!(matches!(reply.command, Some(Command::Enable)));
        assert_eq!(reply.result, Ok(Delivery::Completed));
    }
    assert_eq!(fixture.worker.outstanding(), 0);
    fixture.terminal();
    assert_eq!(fixture.worker.outstanding(), 0);
    assert_eq!(finish(&mut fixture.worker).reason, Reason::InputFailed);
    assert!(fixture.trace.lock().unwrap().sent.is_empty());
}

#[test]
fn failure_observed_inside_pending_query_does_not_enter_infinite_recv() {
    let mut fixture = Harness::new(0);
    fixture.pending_key();
    fixture
        .state
        .finish_in_pending
        .store(true, Ordering::Release);
    fixture.resume.open();
    fixture.terminal();
    assert_eq!(finish(&mut fixture.worker).reason, Reason::InputFailed);
    assert_eq!(fixture.trace.lock().unwrap().released, 1);
}

#[test]
fn stop_racing_an_already_observed_failure_keeps_the_real_error() {
    let mut fixture = Harness::new(0);
    fixture.pending_key();
    fixture.worker.stop();
    fixture.reject();
    fixture.terminal();
    assert_eq!(finish(&mut fixture.worker).reason, Reason::InputFailed);
}

#[test]
fn controller_idle_failure_disables_and_keeps_its_cleanup_receipt() {
    let state = Arc::new(State::default());
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut controller = Controller::new(Probe {
        inner: Mock(trace.clone()),
        state: state.clone(),
        gate: None,
        arm_at: 0,
        rejected: false,
    });
    controller.enable().unwrap();
    controller
        .dispatch(
            1,
            Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    state.reject.store(true, Ordering::Release);
    assert_eq!(
        controller.idle(),
        Err(Error::Backend("asynchronous delivery rejected"))
    );
    assert!(!controller.enabled());
    assert_eq!(controller.last_cleanup().released, 1);
    assert_eq!(trace.lock().unwrap().released, 1);
    assert_eq!(controller.last_sequence(), 1);
}

#[test]
fn failed_result_notification_keeps_the_original_cleanup_not_a_second_empty_cancel() {
    let state = Arc::new(State::default());
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut controller = Controller::new(Probe {
        inner: Mock(trace.clone()),
        state: state.clone(),
        gate: None,
        arm_at: 0,
        rejected: false,
    });
    controller.enable().unwrap();
    controller
        .dispatch(
            1,
            Event::Key {
                code: 65,
                down: true,
            },
        )
        .unwrap();
    state.reject.store(true, Ordering::Release);
    let (_sender, commands) = mpsc::sync_channel(CAPACITY);
    let (results, replies) = mpsc::sync_channel(CAPACITY + 1);
    drop(replies);
    let raw = unsafe { CreateEventW(None, true, false, None) }.unwrap();
    let wake = unsafe { OwnedHandle::from_raw_handle(raw.0) };
    let exit = run(
        controller,
        commands,
        results,
        &AtomicBool::new(false),
        &wake,
        None,
    );
    assert_eq!(exit.reason, Reason::WakeFailed);
    assert_eq!(
        exit.cleanup,
        Some(Cleanup {
            released: 1,
            ..Default::default()
        })
    );
    assert_eq!(trace.lock().unwrap().released, 1);
}
