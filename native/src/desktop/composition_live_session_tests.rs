use super::*;
use crate::desktop::Apartment;
use std::{
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::WaitForSingleObject,
};

#[test]
fn source_close_is_not_worker_completion_and_error_survives_window_close() {
    let _apartment = Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let (closed, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let (job, window) = LiveJob::<Texture>::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
        move |source, _| {
            source.close();
            closed.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(3)).unwrap();
            Err("injected capture failure".into())
        },
    )
    .unwrap();
    let mut session = Session::new(job, window);
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(session.poll().is_none());
    assert!(session.window.is_none());
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(session.completion_handle().unwrap().as_raw_handle()),
                0,
            )
        },
        WAIT_TIMEOUT
    );
    release.send(()).unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(session.completion_handle().unwrap().as_raw_handle()),
                2000,
            )
        },
        WAIT_OBJECT_0
    );
    assert_eq!(session.poll(), Some(Err("injected capture failure".into())));
    assert!(session.poll().is_none());
    assert!(session.completion_handle().is_none());
    assert_eq!(active.load(Ordering::Acquire), 0);
}

#[test]
fn explicit_stop_releases_window_and_wakes_idle_worker_without_detaching() {
    let _apartment = Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let (entered, ready) = mpsc::channel();
    let (job, window) = LiveJob::<Texture>::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
        move |source, _| {
            let (wake, wait) = mpsc::channel();
            source
                .on_idle(Arc::new(move || {
                    let _ = wake.send(());
                }))
                .unwrap();
            entered.send(()).unwrap();
            wait.recv().unwrap();
            Ok(())
        },
    )
    .unwrap();
    let mut session = Session::new(job, window);
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    session.stop();
    assert!(session.window.is_none());
    assert!(session.show().is_err());
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(session.completion_handle().unwrap().as_raw_handle()),
                2000,
            )
        },
        WAIT_OBJECT_0
    );
    assert_eq!(session.poll(), Some(Ok(())));
    assert_eq!(active.load(Ordering::Acquire), 0);
}

#[test]
fn session_keeps_input_owner_after_capture_completion_until_cleanup_finishes() {
    use crate::{
        live_input::{Backend, Event},
        live_input_worker::{Command, Worker},
    };
    struct BlockingRelease(mpsc::SyncSender<()>, mpsc::Receiver<()>);
    impl Backend for BlockingRelease {
        type Target = u8;
        type Error = crate::live_window_target::input_backend::Error;
        fn preflight(&mut self) -> Result<(), Self::Error> {
            Ok(())
        }
        fn resolve(&mut self, _: Event, _: Option<u8>) -> Result<u8, Self::Error> {
            Ok(1)
        }
        fn send(&mut self, _: u8, _: Event, _: u8, _: u32) -> Result<(), Self::Error> {
            Ok(())
        }
        fn cleanup_target(&mut self, target: u8) -> Option<u8> {
            Some(target)
        }
        fn release(&mut self, _: u8, _: Event, _: u8) -> Result<(), Self::Error> {
            self.0.send(()).unwrap();
            self.1.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        }
    }
    let _apartment = Apartment::new().unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let (capture_ready, capture_wait) = mpsc::sync_channel(1);
    let (job, window) = LiveJob::<Texture>::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, 128, 128).map_err(io::Error::other),
        move |source, _| {
            let (tx, rx) = mpsc::channel();
            source
                .on_idle(Arc::new(move || {
                    let _ = tx.send(());
                }))
                .unwrap();
            capture_ready.send(()).unwrap();
            rx.recv().unwrap();
            Ok(())
        },
    )
    .unwrap();
    let mut session = Session::new(job, window);
    capture_wait.recv_timeout(Duration::from_secs(2)).unwrap();
    // 没有窗口授权的区域/测试会话不能通过公开入口开启控制。
    assert_eq!(
        session.enable_input().unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    let (entered, ready) = mpsc::sync_channel(1);
    let (release, gate) = mpsc::sync_channel(1);
    let mut worker = Worker::start_for_ui(move || Ok(BlockingRelease(entered, gate))).unwrap();
    worker.submit(Command::Enable).unwrap();
    worker
        .submit(Command::Input {
            sequence: 1,
            event: Event::Key {
                code: 65,
                down: true,
            },
        })
        .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while worker.outstanding() > 0 {
        if let Some(reply) = worker.poll_reply().unwrap() {
            reply.result.unwrap();
        } else {
            assert!(std::time::Instant::now() < deadline);
            unsafe {
                WaitForSingleObject(HANDLE(worker.wake_handle().as_raw_handle()), 100);
            }
        }
    }
    session.input = Some(worker);
    session.input_enabled = true;
    session.stop();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(session.completion_handle().unwrap().as_raw_handle()),
                2000,
            )
        },
        WAIT_OBJECT_0
    );
    assert!(session.poll().is_none());
    assert!(!session.input_enabled());
    assert!(session.completion_handle().is_none());
    assert!(session.input_completion_handle().is_some());
    release.send(()).unwrap();
    assert_eq!(
        unsafe {
            WaitForSingleObject(
                HANDLE(session.input_completion_handle().unwrap().as_raw_handle()),
                2000,
            )
        },
        WAIT_OBJECT_0
    );
    assert_eq!(session.poll(), Some(Ok(())));
    assert!(session.input_completion_handle().is_none());
    assert!(session.poll().is_none());
}
