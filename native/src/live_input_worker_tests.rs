use super::*;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use windows::Win32::{Foundation::*, System::Threading::WaitForSingleObject};
#[derive(Default)]
struct Trace {
    sent: Vec<Event>,
    released: usize,
    thread: Option<std::thread::ThreadId>,
}
struct Mock(Arc<Mutex<Trace>>);
impl Backend for Mock {
    type Target = u8;
    type Error = &'static str;
    fn preflight(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
    fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Self::Error> {
        Ok(1)
    }
    fn send(&mut self, _: u8, event: Event, _: u8, _: u32) -> Result<(), Self::Error> {
        let mut trace = self.0.lock().unwrap();
        trace.thread = Some(std::thread::current().id());
        trace.sent.push(event);
        Ok(())
    }
    fn cleanup_target(&mut self, target: u8) -> Option<u8> {
        Some(target)
    }
    fn release(&mut self, _: u8, _: Event, _: u8) -> Result<(), Self::Error> {
        self.0.lock().unwrap().released += 1;
        Ok(())
    }
}
fn wait_reply(worker: &mut Worker<&'static str>) -> Reply<&'static str> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(reply) = worker.poll_reply().unwrap() {
            return reply;
        }
        assert!(Instant::now() < deadline);
        let result =
            unsafe { WaitForSingleObject(HANDLE(worker.wake_handle().as_raw_handle()), 100) };
        assert!(result == WAIT_OBJECT_0 || result == WAIT_TIMEOUT);
    }
}
fn finish(worker: &mut Worker<&'static str>) -> Exit {
    let result = unsafe {
        WaitForSingleObject(
            HANDLE(worker.completion_handle().unwrap().as_raw_handle()),
            5000,
        )
    };
    assert_eq!(result, WAIT_OBJECT_0);
    worker.take_exit().unwrap()
}
#[test]
fn commands_execute_off_owner_thread_and_stop_releases_before_join() {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let state = trace.clone();
    let mut worker = Worker::start(move || Ok(Mock(state))).unwrap();
    worker.submit(Command::Enable).unwrap();
    assert!(wait_reply(&mut worker).result.is_ok());
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        })
        .unwrap();
    assert!(wait_reply(&mut worker).result.is_ok());
    assert_eq!(worker.outstanding(), 0);
    assert_ne!(
        trace.lock().unwrap().thread.unwrap(),
        std::thread::current().id()
    );
    worker.stop();
    let exit = finish(&mut worker);
    assert_eq!(exit.reason, Reason::Stopped);
    assert_eq!(exit.cleanup.unwrap().released, 1);
    assert_eq!(trace.lock().unwrap().released, 1);
    assert!(worker.completion_handle().is_none());
    assert!(worker.take_exit().is_none());
}
#[test]
fn full_queue_revokes_and_never_executes_queued_edges() {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let state = trace.clone();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (go_tx, go_rx) = mpsc::sync_channel(1);
    let mut worker = Worker::start(move || {
        ready_tx.send(()).unwrap();
        go_rx.recv().unwrap();
        Ok(Mock(state))
    })
    .unwrap();
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..CAPACITY {
        worker.submit(Command::Enable).unwrap();
    }
    let rejected = worker.submit(Command::Input {
        sequence: 1,
        event: Event::Key {
            code: 65,
            down: true,
        },
    });
    // 先放行夹具，断言失败也不会在 Drop 中等待一个尚未释放的测试闸门。
    go_tx.send(()).unwrap();
    assert_eq!(rejected, Err(SubmitError::Full));
    assert!(worker.stopping());
    assert_eq!(finish(&mut worker).reason, Reason::Stopped);
    assert!(trace.lock().unwrap().sent.is_empty());
}
#[test]
fn dispatch_error_preserves_cleanup_report_and_terminates() {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let state = trace.clone();
    let mut worker = Worker::start(move || Ok(Mock(state))).unwrap();
    worker.submit(Command::Enable).unwrap();
    wait_reply(&mut worker).result.unwrap();
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        })
        .unwrap();
    wait_reply(&mut worker).result.unwrap();
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 66,
                down: true,
            },
        })
        .unwrap();
    assert_eq!(wait_reply(&mut worker).result, Err(Error::Sequence));
    let exit = finish(&mut worker);
    assert_eq!(exit.reason, Reason::InputFailed);
    assert_eq!(exit.cleanup.unwrap().released, 1);
    assert_eq!(trace.lock().unwrap().sent.len(), 1);
}
#[test]
fn factory_error_panic_and_idle_drop_are_reaped() {
    let mut worker = Worker::start(|| Err::<Mock, _>("fixture initialization".into())).unwrap();
    assert_eq!(
        finish(&mut worker).reason,
        Reason::Initialization("fixture initialization".into())
    );
    let mut worker = Worker::start(|| -> Result<Mock, String> { panic!("fixture panic") }).unwrap();
    let exit = finish(&mut worker);
    assert_eq!(exit.reason, Reason::Panicked);
    assert_eq!(exit.cleanup, None);
    let trace = Arc::new(Mutex::new(Trace::default()));
    let worker = Worker::start(move || Ok(Mock(trace))).unwrap();
    drop(worker);
}

#[test]
fn worker_slot_limit_is_released_without_spawning_threads() {
    let slots: Vec<_> = (0..16).map(|_| Slot::acquire().unwrap()).collect();
    assert_eq!(
        Slot::acquire().err().unwrap().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(slots);
    drop(Slot::acquire().unwrap());
}

#[test]
fn ui_mode_posts_reply_wakeup_before_worker_exit() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut worker = Worker::start_for_ui(move || Ok(Mock(trace))).unwrap();
    worker.submit(Command::Enable).unwrap();
    wait_reply(&mut worker).result.unwrap();
    unsafe {
        assert_eq!(
            MsgWaitForMultipleObjectsEx(None, 2000, QS_ALLINPUT, MWMO_INPUTAVAILABLE),
            WAIT_OBJECT_0
        );
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool());
        assert_eq!(message.message, WM_NULL);
        assert_eq!(
            WaitForSingleObject(
                HANDLE(worker.completion_handle().unwrap().as_raw_handle()),
                0
            ),
            WAIT_TIMEOUT
        );
    }
    worker.stop();
    assert_eq!(finish(&mut worker).reason, Reason::Stopped);
}

#[test]
fn window_stop_signal_wakes_idle_worker_without_owner_poll_or_sender_drop() {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let state = trace.clone();
    let mut worker = Worker::start(move || Ok(Mock(state))).unwrap();
    worker.submit(Command::Enable).unwrap();
    wait_reply(&mut worker).result.unwrap();
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        })
        .unwrap();
    wait_reply(&mut worker).result.unwrap();
    let signal = worker.stop_signal().unwrap();
    signal.request();
    assert!(worker.stopping());
    assert_eq!(finish(&mut worker).cleanup.unwrap().released, 1);
    assert_eq!(trace.lock().unwrap().released, 1);
    drop(signal);
}
