//! 验证真实 Win32 等待和消息泵；不连接网络，不展示窗口。
use super::*;
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use windows::Win32::System::Threading::CreateEventW;

fn event() -> OwnedHandle {
    let handle = unsafe { CreateEventW(None, true, false, None) }.unwrap();
    unsafe { OwnedHandle::from_raw_handle(handle.0) }
}

#[test]
fn empty_shutdown_does_not_wait_or_create_a_worker() {
    let mut calls = 0;
    pump_until(Instant::now() + GRACE, || {
        calls += 1;
        Ok(Vec::new())
    })
    .unwrap();
    assert_eq!(calls, 1);
}

#[test]
fn shutdown_deadline_does_not_reenter_settlement_or_replay_requests() {
    let event = event();
    let mut calls = 0;
    let deadline = Instant::now() + Duration::from_millis(20);
    let result = pump_until(deadline, || {
        calls += 1;
        Ok(vec![HANDLE(event.as_raw_handle())])
    });
    assert!(result.is_err());
    assert!(Instant::now() >= deadline);
    // 系统可发送窗口消息；这里只要求截止后不再调用 settle 发起新工作。
    let mut after_deadline = false;
    assert!(pump_until(deadline, || {
        after_deadline = true;
        Ok(Vec::new())
    })
    .is_err());
    assert!(calls >= 1);
    assert!(!after_deadline);
}

#[test]
fn shutdown_preserves_quit_on_timeout_and_settlement_error() {
    let event = event();
    unsafe { PostQuitMessage(19) };
    assert!(pump_until(Instant::now() + Duration::from_millis(20), || {
        Ok(vec![HANDLE(event.as_raw_handle())])
    })
    .is_err());
    let mut message = MSG::default();
    assert!(unsafe { PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE) }.as_bool());
    assert_eq!(message.wParam.0, 19);
    unsafe { PostQuitMessage(23) };
    let mut first = true;
    assert!(pump_until(Instant::now() + GRACE, || {
        if std::mem::take(&mut first) {
            Ok(vec![HANDLE(event.as_raw_handle())])
        } else {
            Err(Error::new(E_FAIL, "fixture settlement failed"))
        }
    })
    .is_err());
    assert!(unsafe { PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE) }.as_bool());
    assert_eq!(message.wParam.0, 23);
}

#[test]
fn shutdown_pumps_worker_synchronous_messages_before_joining() {
    use crate::desktop::{Apartment, Window};
    let _com = Apartment::new().unwrap();
    let window = Window(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("grace"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap()
    });
    let hwnd = window.0 .0 as usize;
    let mut worker = Some(std::thread::spawn(move || {
        let mut length = 0;
        let sent = unsafe {
            SendMessageTimeoutW(
                HWND(hwnd as *mut _),
                WM_GETTEXTLENGTH,
                WPARAM(0),
                LPARAM(0),
                SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
                3000,
                Some(&mut length),
            )
        };
        assert_ne!(sent.0, 0);
        assert_eq!(length, 5);
    }));
    pump_until(Instant::now() + Duration::from_secs(5), || {
        if worker.as_ref().unwrap().is_finished() {
            worker.take().unwrap().join().unwrap();
            Ok(Vec::new())
        } else {
            Ok(vec![HANDLE(worker.as_ref().unwrap().as_raw_handle())])
        }
    })
    .unwrap();
    assert!(worker.is_none());
}
