//! 父源保护覆盖附属输入收尾；真实 Worker/假后端，不运行 WGC、DLL 或源输入。
use super::*;
#[path = "composition_live_popup_cleanup_tests.rs"]
mod cleanup_tests;
use crate::{
    live_input::Event,
    live_input_worker::{Command, Worker},
    live_visibility_owner::Owner,
};
use std::{
    io,
    os::windows::io::AsRawHandle,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
};
use windows::Win32::System::Threading::WaitForSingleObject;

fn session(active: &Arc<AtomicUsize>) -> Session {
    let (entered, ready) = mpsc::sync_channel(1);
    let (job, window) = LiveJob::spawn_bound(
        active,
        |consumer| LiveWindow::new(consumer, 100, 100).map_err(io::Error::other),
        move |frames, _| {
            let (wake, wait) = mpsc::channel();
            frames
                .on_idle(Arc::new(move || {
                    let _ = wake.send(());
                }))
                .unwrap();
            entered.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        },
    )
    .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    Session::new(job, window)
}
struct BlockingRelease(mpsc::SyncSender<()>, mpsc::Receiver<()>);
impl crate::live_input::Backend for BlockingRelease {
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
fn child_input(child: &mut Session) -> (mpsc::Receiver<()>, mpsc::SyncSender<()>) {
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
    let deadline = Instant::now() + Duration::from_secs(2);
    while worker.outstanding() > 0 {
        if let Some(reply) = worker.poll_reply().unwrap() {
            reply.result.unwrap();
        } else {
            assert!(Instant::now() < deadline);
            unsafe {
                WaitForSingleObject(HANDLE(worker.wake_handle().as_raw_handle()), 100);
            }
        }
    }
    child.input = Some(worker);
    child.input_enabled = true;
    (ready, release)
}
fn finish_visibility(owner: &mut Owner) {
    owner.close();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !owner.finished() {
        owner.poll().unwrap();
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn retains_protection(close_window: bool) {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let source = Source::new(None, rect(-30000, -30000, 80, 60), false);
    let mut visibility = Owner::fixture();
    let active = Arc::new(AtomicUsize::new(0));
    let mut root = session(&active);
    root.protect(visibility.acquire(Target::bind(source.0).unwrap()).unwrap());
    let mut child = session(&active);
    let (releasing, release) = child_input(&mut child);
    root.popups.members.push(Member {
        hwnd: 0,
        placement: rect(0, 0, 100, 100),
        session: child,
    });
    if close_window {
        // 只关闭自有 B，覆盖非显式 Session::stop 的回收分支。
        unsafe {
            SendMessageW(
                root.window.as_ref().unwrap().popup_hwnd(),
                WM_CLOSE,
                None,
                None,
            );
        }
        assert!(root.poll().is_none());
    } else {
        root.stop();
    }
    releasing.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(root.input.is_none());
    assert!(
        root.protection.is_some(),
        "child input still owns source cleanup"
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while root.capture_result.is_none() {
        assert!(root.poll().is_none());
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(root.window.is_none());
    assert!(
        root.protection.is_some(),
        "parent capture completion cannot release child protection"
    );
    assert!(root.input_completion_handle().is_some());
    release.send(()).unwrap();
    let handle = root.input_completion_handle().unwrap();
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 3000) },
        WAIT_OBJECT_0
    );
    let result = loop {
        if let Some(result) = root.poll() {
            break result;
        }
        assert!(Instant::now() < deadline + Duration::from_secs(3));
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(result, Ok(()));
    assert!(root.protection.is_none());
    assert!(root.popups.is_empty());
    assert!(root.input_completion_handle().is_none());
    assert_eq!(active.load(Ordering::Acquire), 0);
    finish_visibility(&mut visibility);
}

#[test]
fn explicit_stop_keeps_parent_protection_until_child_input_release_finishes() {
    retains_protection(false);
}
#[test]
fn closing_b_keeps_parent_protection_until_child_input_release_finishes() {
    retains_protection(true);
}
#[test]
fn a_session_without_input_does_not_wait_for_capture_to_release_protection() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let source = Source::new(None, rect(-30000, -30000, 80, 60), false);
    let mut visibility = Owner::fixture();
    let active = Arc::new(AtomicUsize::new(0));
    let (entered, ready) = mpsc::sync_channel(1);
    let (release, gate) = mpsc::sync_channel(1);
    let (job, window) = LiveJob::spawn_bound(
        &active,
        |consumer| LiveWindow::new(consumer, 100, 100).map_err(io::Error::other),
        move |_, _| {
            entered.send(()).unwrap();
            gate.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        },
    )
    .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut root = Session::new(job, window);
    root.protect(visibility.acquire(Target::bind(source.0).unwrap()).unwrap());
    root.stop();
    assert!(root.protection.is_none());
    assert!(root.input_completion_handle().is_none());
    assert_eq!(
        unsafe {
            WaitForSingleObject(HANDLE(root.completion_handle().unwrap().as_raw_handle()), 0)
        },
        WAIT_TIMEOUT,
        "capture is still blocked when protection is released"
    );
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = root.poll() {
            result.unwrap();
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(active.load(Ordering::Acquire), 0);
    finish_visibility(&mut visibility);
}
