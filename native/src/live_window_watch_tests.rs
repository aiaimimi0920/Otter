use super::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use windows::core::w;
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                let _ = DestroyWindow(self.0);
            }
        }
    }
}
#[test]
fn real_hidden_window_destroy_notifies_once_and_retired_hook_ignores_late_events() {
    unsafe {
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Live watcher fixture"),
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
        .unwrap();
        let _window = Window(hwnd);
        let identity = super::super::identity(hwnd).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let watcher = Watcher::install(
            identity,
            Arc::new(move || {
                observed.fetch_add(1, Ordering::AcqRel);
            }),
        )
        .unwrap();
        let hook = watcher.hook;
        on_event(
            hook,
            EVENT_OBJECT_DESTROY,
            hwnd,
            OBJID_WINDOW.0,
            1,
            identity.thread,
            0,
        );
        assert_eq!(count.load(Ordering::Acquire), 0);
        DestroyWindow(hwnd).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while count.load(Ordering::Acquire) == 0 {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                DispatchMessageW(&message);
            }
            assert!(Instant::now() < deadline, "destroy event did not arrive");
            std::thread::sleep(Duration::from_millis(1));
        }
        on_event(
            hook,
            EVENT_OBJECT_DESTROY,
            hwnd,
            OBJID_WINDOW.0,
            0,
            identity.thread,
            0,
        );
        assert_eq!(count.load(Ordering::Acquire), 1);
        drop(watcher);
        on_event(
            hook,
            EVENT_OBJECT_DESTROY,
            hwnd,
            OBJID_WINDOW.0,
            0,
            identity.thread,
            0,
        );
        assert_eq!(count.load(Ordering::Acquire), 1);
        assert!(ENTRIES.with(|entries| entries.borrow().is_empty()));
    }
}
