//! 消息专用窗口测试不显示界面，也不修改用户当前前景窗口。
use super::*;
use windows::core::w;

struct Destination(HWND);
impl Drop for Destination {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

fn destination() -> Destination {
    Destination(unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Otter priority test"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
        .unwrap()
    })
}

fn notifications(hwnd: HWND) -> usize {
    let mut count = 0;
    let mut message = MSG::default();
    unsafe {
        while PeekMessageW(
            &mut message,
            Some(hwnd),
            PRIORITY_CHANGED,
            PRIORITY_CHANGED,
            PM_REMOVE,
        )
        .as_bool()
        {
            count += 1;
        }
    }
    count
}

#[test]
fn hooks_coalesce_notifications_and_release_thread_registration() {
    let window = destination();
    let watcher = Watcher::install(window.0).unwrap();
    assert_eq!(watcher._hooks.hooks.len(), 2);
    assert!(Watcher::install(window.0).is_err());
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            window.0,
            0,
            0,
            0,
            0,
        );
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            window.0,
            0,
            0,
            0,
            0,
        );
    }
    assert_eq!(notifications(window.0), 1);
    acknowledge(window.0);
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            window.0,
            0,
            0,
            0,
            0,
        );
    }
    assert_eq!(notifications(window.0), 1);
    drop(watcher);
    assert!(DESTINATIONS.with(|targets| targets.borrow().is_empty()));
    assert!(SHARED.with(|shared| shared.borrow().upgrade().is_none()));
    let _replacement = Watcher::install(window.0).unwrap();
}

#[test]
fn invalid_destination_does_not_poison_registration() {
    assert!(Watcher::install(HWND::default()).is_err());
    assert!(DESTINATIONS.with(|targets| targets.borrow().is_empty()));
    let window = destination();
    let _watcher = Watcher::install(window.0).unwrap();
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_OBJECT_SHOW,
            window.0,
            OBJID_WINDOW.0,
            0,
            0,
            0,
        );
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_OBJECT_SHOW,
            HWND::default(),
            OBJID_CLIENT.0,
            0,
            0,
            0,
        );
    }
    assert!(DESTINATIONS.with(|targets| targets.borrow().iter().all(|target| !target.1)));
    // PeekMessage 会分发系统 WinEvent 回调。先核对忽略路径，再注销真实 hook，
    // 否则其他测试/桌面的窗口事件可在读取空队列时设置 PENDING，污染此断言。
    drop(_watcher);
    assert_eq!(notifications(window.0), 0);
}

#[test]
fn two_windows_share_hooks_but_acknowledge_and_release_independently() {
    let first = destination();
    let second = destination();
    let first_watch = Watcher::install(first.0).unwrap();
    let second_watch = Watcher::install(second.0).unwrap();
    assert!(Rc::ptr_eq(&first_watch._hooks, &second_watch._hooks));
    assert_eq!(second_watch._hooks.hooks.len(), 2);
    let owner = Rc::downgrade(&second_watch._hooks);
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            HWND::default(),
            0,
            0,
            0,
            0,
        );
    }
    assert_eq!(notifications(first.0), 1);
    assert_eq!(notifications(second.0), 1);
    acknowledge(first.0);
    // 第一窗口确认不能清除第二窗口的 pending，否则重复系统事件会再次排队。
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            HWND::default(),
            0,
            0,
            0,
            0,
        );
    }
    assert_eq!(notifications(first.0), 1);
    assert_eq!(notifications(second.0), 0);
    drop(first_watch);
    assert!(owner.upgrade().is_some());
    acknowledge(second.0);
    unsafe {
        on_event(
            HWINEVENTHOOK::default(),
            EVENT_SYSTEM_FOREGROUND,
            HWND::default(),
            0,
            0,
            0,
            0,
        );
    }
    assert_eq!(notifications(second.0), 1);
    drop(second_watch);
    assert!(owner.upgrade().is_none());
    assert!(DESTINATIONS.with(|targets| targets.borrow().is_empty()));
}

#[test]
fn host_destroy_dispatch_releases_its_subscription_before_state_drop() {
    let window = destination();
    let state = super::super::State::new(None, false).unwrap();
    let watcher = Watcher::install(window.0).unwrap();
    let hooks = Rc::downgrade(&watcher._hooks);
    *state.priority_watcher.borrow_mut() = Some(watcher);
    // 直接调用生产销毁路由，不登记宿主计数，不向测试线程发送 WM_QUIT。
    unsafe {
        super::super::host_lifetime::dispatch(window.0, &state, WM_DESTROY, WPARAM(0), LPARAM(0));
    }
    assert!(state.priority_watcher.borrow().is_none());
    assert!(hooks.upgrade().is_none());
    assert!(DESTINATIONS.with(|targets| targets.borrow().is_empty()));
}
