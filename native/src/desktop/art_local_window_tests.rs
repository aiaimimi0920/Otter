//! 隐藏真实 HWND 首帧呈现与宿主换代后撤销/回收；不抢前台，不冒充完整节点或交互验收。
use super::*;
#[path = "art_local_window_input_tests.rs"]
mod input_tests;
use std::{cell::Cell, os::windows::io::AsRawHandle, time::Duration};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForMultipleObjects,
};
fn wait(owner: &Window, index: usize) {
    let handles: Vec<_> = owner
        .wait_handles()
        .map(|handle| HANDLE(handle.as_raw_handle()))
        .collect();
    assert_eq!(
        unsafe { WaitForMultipleObjects(&handles, false, 8000) }.0,
        WAIT_OBJECT_0.0 + index as u32
    );
}
#[test]
fn initial_frame_builds_real_hidden_window_then_host_removal_stops_original_child() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let mut owner = Window::new(runtime, crate::surface_javascript::fixture::binding());
    wait(&owner, 1);
    owner.poll(|| true, false).unwrap();
    let window = owner.window.as_ref().unwrap();
    assert!(window.is_alive());
    assert!(!unsafe {
        windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(window.dialog_owner().unwrap())
    }
    .as_bool());
    owner.poll(|| false, false).unwrap();
    assert!(!owner.window.as_ref().unwrap().is_alive());
    wait(&owner, 0);
    owner.poll(|| false, false).unwrap();
    assert!(owner.finished());
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
#[test]
fn host_change_during_initial_window_commit_closes_candidate_and_never_shows_it() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let mut owner = Window::new(runtime, crate::surface_javascript::fixture::binding());
    wait(&owner, 1);
    let calls = Cell::new(0);
    assert!(owner
        .poll(
            || {
                let n = calls.get() + 1;
                calls.set(n);
                n < 3
            },
            false
        )
        .is_err());
    assert!(owner.stopping);
    assert!(!owner.window.as_ref().unwrap().is_alive());
    wait(&owner, 0);
    owner.poll(|| false, false).unwrap();
    assert!(owner.finished());
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}

#[test]
fn hidden_initial_frame_is_shown_once_without_another_reply() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let mut owner = Window::new(runtime, crate::surface_javascript::fixture::binding());
    wait(&owner, 1);
    owner.poll(|| true, false).unwrap();
    assert!(!owner.shown);
    let hwnd = owner.window.as_ref().unwrap().dialog_owner().unwrap();
    use windows::Win32::UI::WindowsAndMessaging::{
        IsWindowVisible, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
        SW_HIDE,
    };
    // 屏外非激活窗口验证可见性，不覆盖用户当前前台。
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            -30000,
            -30000,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
        .unwrap();
    }
    owner.poll(|| true, true).unwrap();
    assert!(owner.shown);
    assert!(unsafe { IsWindowVisible(hwnd) }.as_bool());
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    owner.poll(|| true, true).unwrap();
    assert!(!unsafe { IsWindowVisible(hwnd) }.as_bool());
    owner.stop().unwrap();
    wait(&owner, 0);
    owner.poll(|| false, false).unwrap();
    assert!(owner.finished());
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}

#[test]
fn native_confirmation_click_cannot_bypass_changed_host_before_handoff() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let (runtime, _grant, probe) = crate::surface_javascript::signed::runtime::tests::start(None);
    let mut owner = Window::new(runtime, crate::surface_javascript::fixture::binding());
    wait(&owner, 1);
    owner.poll(|| true, false).unwrap();
    let channel = crate::live_relay_http::art_prepare::service::confirmation::Channel::new(
        std::sync::Arc::new(|| {}),
    );
    let ticket = channel
        .publish(
            "host confirmation".into(),
            std::time::Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
    let prompt = confirmation_dialog::Prompt::hidden(
        owner.window.as_ref().unwrap().dialog_owner().unwrap(),
        ticket.clone(),
    )
    .unwrap();
    let hwnd = prompt.handle().unwrap();
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd,
            windows::Win32::UI::WindowsAndMessaging::WM_COMMAND,
            Some(windows::Win32::Foundation::WPARAM(1)),
            Some(windows::Win32::Foundation::LPARAM(0)),
        );
    }
    owner.confirmation = Some(prompt);
    assert!(
        !owner.session_end_idle(),
        "pending trusted confirmation blocks system admission"
    );
    owner.poll_confirmation(&|| false, true).unwrap();
    assert_eq!(ticket.choice(), Some(false));
    assert!(owner.confirmation.is_none());
    assert!(!unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(hwnd)) }.as_bool());
    owner.stop().unwrap();
    wait(&owner, 0);
    owner.poll(|| false, false).unwrap();
    assert!(probe.recv_timeout(Duration::from_secs(5)).unwrap().exited());
}
